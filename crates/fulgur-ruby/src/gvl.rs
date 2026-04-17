//! GVL (Giant VM Lock) 解放ヘルパ。
//!
//! magnus 0.7 は GVL 解放 API を持たないため、rb-sys の
//! `rb_thread_call_without_gvl` を直接呼ぶ。解放中のスレッドで
//! Ruby VM に触れると UB になるため、closure は Rust-only データで動く
//! ようにする (`&Engine`, `String` など)。
//!
//! # 設計メモ
//!
//! - `body` は `fn` pointer (キャプチャ無し)。キャプチャが必要なら
//!   `Data` に必要な値を詰めて渡す。現在の `render_html` 呼び出しでは
//!   `Args { engine: *const Engine, html: String }` 相当を渡す。
//! - ubf (unblock function) は `None`。Ruby プロセスが signal を受けても
//!   GVL 再取得まで保留される。v0.5.0 では interruptable rendering を
//!   実装しないので OK。
//! - 返り値 `Ret` は `Option` に包んで途中中断を表現することも考えたが、
//!   現状 ubf 無しで shim は常に完走するため、`Ret` を直接返して内部で
//!   `unwrap()` する。Payload が書き込まれないケース (shim が呼ばれない)
//!   は起こらない前提で panic させる。

use std::ffi::c_void;

/// `body` を GVL 解放状態で実行する。
///
/// # Safety (呼び出し側契約)
///
/// `body` 内で Ruby VM API を呼んではならない。`Value`, `RString`, `Ruby::get()`
/// のような Ruby 依存の型・関数はすべて禁止。純粋な Rust データ (`String`,
/// `&Engine` など) のみ操作すること。
pub fn without_gvl<Data, Ret>(data: Data, body: fn(Data) -> Ret) -> Ret {
    struct Payload<D, R> {
        data: Option<D>,
        body: fn(D) -> R,
        result: Option<R>,
    }

    unsafe extern "C" fn shim<D, R>(arg: *mut c_void) -> *mut c_void {
        // SAFETY: arg は `Payload<D, R>` への mutable pointer (caller が作る)。
        // GVL 解放中、他の Ruby スレッドはこの領域を触れない (payload は
        // caller のスタックにあり、caller は block する)。
        let p = unsafe { &mut *(arg as *mut Payload<D, R>) };
        let data = p.data.take().expect("payload data taken twice");
        p.result = Some((p.body)(data));
        std::ptr::null_mut()
    }

    let mut payload: Payload<Data, Ret> = Payload {
        data: Some(data),
        body,
        result: None,
    };
    // SAFETY: shim は payload を mutable 参照するが、rb_thread_call_without_gvl
    // は caller を block し、shim が終わるまで payload は生存する。ubf=None
    // なので shim は必ず完走し、payload.result が Some になる。
    unsafe {
        rb_sys::rb_thread_call_without_gvl(
            Some(shim::<Data, Ret>),
            &mut payload as *mut _ as *mut c_void,
            None,
            std::ptr::null_mut(),
        );
    }
    payload
        .result
        .expect("rb_thread_call_without_gvl did not invoke shim")
}
