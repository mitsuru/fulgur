# frozen_string_literal: true

require "spec_helper"
require "base64"

RSpec.describe Fulgur::Pdf do
  let(:html) { File.read(File.expand_path("fixtures/simple.html", __dir__)) }
  let(:engine) { Fulgur::Engine.new(page_size: :a4) }
  let(:pdf) { engine.render_html(html) }

  it "returns a Fulgur::Pdf" do
    expect(pdf).to be_a(described_class)
  end

  describe "#bytesize" do
    it "returns a positive integer" do
      expect(pdf.bytesize).to be > 100
    end
  end

  describe "#to_s" do
    it "returns ASCII-8BIT encoded string starting with %PDF-" do
      s = pdf.to_s
      expect(s.encoding).to eq(Encoding::ASCII_8BIT)
      expect(s[0, 5]).to eq("%PDF-".b)
    end

    it "to_s.bytesize == bytesize" do
      expect(pdf.to_s.bytesize).to eq(pdf.bytesize)
    end
  end

  describe "#to_base64" do
    it "returns a String that round-trips via Base64.strict_decode64" do
      b64 = pdf.to_base64
      expect(b64).to match(%r{\A[A-Za-z0-9+/=]+\z})
      decoded = Base64.strict_decode64(b64)
      expect(decoded.force_encoding(Encoding::ASCII_8BIT)).to eq(pdf.to_s)
    end
  end

  describe "#to_data_uri" do
    it "returns data URI with application/pdf prefix" do
      expect(pdf.to_data_uri).to start_with("data:application/pdf;base64,")
    end
  end

  describe "#write_to_path" do
    it "writes binary PDF to file" do
      Dir.mktmpdir do |dir|
        path = File.join(dir, "out.pdf")
        pdf.write_to_path(path)
        expect(File.binread(path)).to eq(pdf.to_s)
      end
    end

    it "raises an IO-related error for invalid path" do
      expect { pdf.write_to_path("/nonexistent/dir/out.pdf") }
        .to raise_error(StandardError)
    end
  end

  describe "#write_to_io" do
    it "writes binary PDF to StringIO" do
      io = StringIO.new
      pdf.write_to_io(io)
      expect(io.string.force_encoding(Encoding::ASCII_8BIT)).to eq(pdf.to_s)
    end

    it "total bytes written equals pdf.bytesize" do
      io = StringIO.new
      pdf.write_to_io(io)
      expect(io.string.bytesize).to eq(pdf.bytesize)
    end

    it "calls binmode on the IO object" do
      io = StringIO.new
      # StringIO responds to binmode (noop on Ruby 3+ but defined).
      expect(io).to receive(:binmode).at_least(:once).and_call_original
      allow(io).to receive(:write).and_call_original
      pdf.write_to_io(io)
    end
  end
end
