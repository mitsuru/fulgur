# frozen_string_literal: true

require "spec_helper"

RSpec.describe Fulgur::Engine do
  let(:html) { File.read(File.expand_path("fixtures/simple.html", __dir__)) }

  describe ".new" do
    it "accepts no kwargs" do
      expect { described_class.new }.not_to raise_error
    end

    it "accepts page_size as string" do
      expect { described_class.new(page_size: "A4") }.not_to raise_error
    end

    it "accepts page_size as symbol" do
      expect { described_class.new(page_size: :a4) }.not_to raise_error
    end

    it "accepts page_size as PageSize constant" do
      expect { described_class.new(page_size: Fulgur::PageSize::A4) }.not_to raise_error
    end

    it "raises ArgumentError for unknown page_size string" do
      expect { described_class.new(page_size: "XYZ") }.to raise_error(ArgumentError)
    end

    it "accepts margin kwarg" do
      expect { described_class.new(margin: Fulgur::Margin.uniform(50)) }.not_to raise_error
    end

    it "accepts assets kwarg" do
      bundle = Fulgur::AssetBundle.new
      bundle.css "body { color: red }"
      expect { described_class.new(assets: bundle) }.not_to raise_error
    end
  end

  describe ".builder" do
    it "returns an EngineBuilder" do
      expect(described_class.builder).to be_a(Fulgur::EngineBuilder)
    end

    it "builds an Engine via chain" do
      engine = described_class.builder.page_size(:a4).build
      expect(engine).to be_a(described_class)
    end

    it "supports full chain" do
      engine = described_class.builder
        .page_size(:letter)
        .margin(Fulgur::Margin.uniform(72))
        .landscape(true)
        .title("test")
        .build
      expect(engine).to be_a(described_class)
    end

    it "raises RuntimeError on double build" do
      b = described_class.builder
      b.build
      expect { b.build }.to raise_error(/already been built/)
    end
  end

  describe "GVL release" do
    # render_html が GVL を解放している限り、Ruby 側の別スレッド (ticker) は
    # レンダリング中にも進行できる。GVL が解放されていない場合、ticker は
    # render_html 完了まで走れず counter はほぼ 0 のまま。これは時間ベースの
    # ヒューリスティックであり、極端に遅い CI 環境では counter が伸びにくい
    # 可能性があるため、閾値はかなり保守的に設定している。
    it "allows concurrent Ruby threads during render_html" do
      engine = described_class.new
      html = File.read(File.expand_path("fixtures/simple.html", __dir__))

      counter = 0
      mutex = Mutex.new
      ticker_done = false

      ticker = Thread.new do
        80.times do
          mutex.synchronize { counter += 1 }
          sleep 0.003
        end
        ticker_done = true
      end

      10.times { engine.render_html(html) }
      ticker.join(10)

      expect(ticker_done).to be true
      expect(counter).to be >= 10
    end
  end
end
