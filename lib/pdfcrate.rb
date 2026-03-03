# frozen_string_literal: true

require "pdfcrate/version"
require "pdfcrate/pdfcrate"

module Pdfcrate
  # Prawn::View-compatible mixin: delegates method calls to self.document
  module View
    def document
      @document
    end

    private

    def method_missing(name, *args, **kwargs, &block)
      return super unless document.respond_to?(name)

      if kwargs.empty?
        document.send(name, *args, &block)
      else
        document.send(name, *args, **kwargs, &block)
      end
    end

    def respond_to_missing?(name, include_private = false)
      document.respond_to?(name) || super
    end
  end

  # Extend GridProxy with bounding_box method that delegates to Document
  class GridProxy
    attr_accessor :_document

    def bounding_box(&block)
      raise "GridProxy must be associated with a Document" unless _document

      if is_span
        _document.grid_span_bounding_box(row, col, end_row, end_col, &block)
      else
        _document.grid_cell_bounding_box(row, col, &block)
      end
    end
  end

  class Document
    # Override grid to attach document reference
    alias_method :_grid_raw, :grid

    def grid(*args)
      proxy = _grid_raw(*args)
      proxy._document = self
      proxy
    end

    # Grid cell bounding box - uses grid layout coordinates
    def grid_cell_bounding_box(row, col, &block)
      _grid_cell_bb(row, col, &block)
    end

    def grid_span_bounding_box(r1, c1, r2, c2, &block)
      _grid_span_bb(r1, c1, r2, c2, &block)
    end

    # font_families.update() compatibility
    # Returns a FontFamiliesUpdater that intercepts .update()
    class FontFamiliesUpdater
      def initialize(doc)
        @doc = doc
      end

      def update(families)
        families.each do |name, styles|
          kwargs = {}
          styles.each do |style, path|
            kwargs[style] = path
          end
          @doc.register_font_family(name, **kwargs)
        end
      end
    end

    def font_families
      FontFamiliesUpdater.new(self)
    end
  end
end
