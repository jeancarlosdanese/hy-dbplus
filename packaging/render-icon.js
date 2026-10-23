// Renderiza um SVG em PNG com fundo transparente pelo AppKit (FEAT-014):
//   osascript -l JavaScript packaging/render-icon.js <svg> <png de saída> <lado em px>
// O `qlmanage` pintava fundo branco fora do quadrado arredondado; o NSImage respeita o alfa.
// Usado pelo `cargo xtask bundle` (iconset) e na receita do `packaging/icon-256.png`.
ObjC.import('AppKit');
function run(argv) {
  const [svgPath, outPath, sizeStr] = argv;
  const size = parseInt(sizeStr, 10);
  if (!svgPath || !outPath || !(size > 0)) {
    return 'uso: render-icon.js <svg> <png> <lado>';
  }
  const img = $.NSImage.alloc.initWithContentsOfFile(svgPath);
  if (img.isNil()) {
    return 'não abriu o SVG: ' + svgPath;
  }
  const rep = $.NSBitmapImageRep.alloc.initWithBitmapDataPlanesPixelsWidePixelsHighBitsPerSampleSamplesPerPixelHasAlphaIsPlanarColorSpaceNameBytesPerRowBitsPerPixel(
    null, size, size, 8, 4, true, false, $.NSCalibratedRGBColorSpace, 0, 0);
  $.NSGraphicsContext.saveGraphicsState;
  $.NSGraphicsContext.setCurrentContext($.NSGraphicsContext.graphicsContextWithBitmapImageRep(rep));
  img.drawInRectFromRectOperationFraction($.NSMakeRect(0, 0, size, size), $.NSZeroRect, $.NSCompositingOperationSourceOver, 1.0);
  $.NSGraphicsContext.restoreGraphicsState;
  const png = rep.representationUsingTypeProperties($.NSBitmapImageFileTypePNG, $());
  return png.writeToFileAtomically(outPath, true) ? 'ok' : ('não gravou: ' + outPath);
}
