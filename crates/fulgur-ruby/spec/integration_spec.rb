# frozen_string_literal: true

require "spec_helper"
require "base64"

RSpec.describe "Integration" do
  let(:fixtures) { File.expand_path("fixtures", __dir__) }
  let(:html) { File.read(File.join(fixtures, "simple.html")) }

  it "renders with AssetBundle (font + CSS)" do
    bundle = Fulgur::AssetBundle.new
    bundle.css "body { color: red }"
    bundle.font_file File.join(fixtures, "noto_sans.ttf")
    engine = Fulgur::Engine.new(page_size: :a4, assets: bundle)
    pdf = engine.render_html(html)
    expect(pdf.bytesize).to be > 100
    expect(pdf.to_s[0, 5]).to eq("%PDF-".b)
  end

  it "renders via builder chain + custom margin" do
    engine = Fulgur::Engine.builder
                           .page_size(:letter)
                           .margin(Fulgur::Margin.new(72, 36))
                           .build
    pdf = engine.render_html(html)
    expect(pdf.bytesize).to be > 100
  end

  it "render_html_to_file writes a PDF" do
    engine = Fulgur::Engine.new
    Dir.mktmpdir do |dir|
      path = File.join(dir, "out.pdf")
      engine.render_html_to_file(html, path)
      contents = File.binread(path)
      expect(contents[0, 5]).to eq("%PDF-".b)
      expect(contents.bytesize).to be > 100
    end
  end

  it "Pdf#to_base64 survives a Rust-side round-trip" do
    engine = Fulgur::Engine.new(page_size: :a4)
    pdf = engine.render_html(html)
    decoded = Base64.strict_decode64(pdf.to_base64)
    expect(decoded.force_encoding(Encoding::ASCII_8BIT)).to eq(pdf.to_s)
  end
end
