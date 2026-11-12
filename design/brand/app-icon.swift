// Draws the macOS app icon: the logo on Apple's 824 px rounded square inside a
// transparent 1024 px canvas, so the Dock shows it at the same size as other apps.
// Usage: swift design/brand/app-icon.swift design/brand/logo.png design/brand/app-icon.png
import AppKit

let canvas = 1024.0
let tile = 824.0
let radius = 185.4
let arguments = CommandLine.arguments
guard arguments.count == 3,
      let logo = NSImage(contentsOfFile: arguments[1])?
          .cgImage(forProposedRect: nil, context: nil, hints: nil)
else {
    FileHandle.standardError.write(Data("usage: app-icon.swift <logo.png> <out.png>\n".utf8))
    exit(1)
}

let space = CGColorSpace(name: CGColorSpace.sRGB)!
let context = CGContext(
    data: nil, width: Int(canvas), height: Int(canvas), bitsPerComponent: 8,
    bytesPerRow: 0, space: space, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
let inset = (canvas - tile) / 2
let rect = CGRect(x: inset, y: inset, width: tile, height: tile)
let shape = CGPath(roundedRect: rect, cornerWidth: radius, cornerHeight: radius, transform: nil)

context.saveGState()
context.setShadow(
    offset: CGSize(width: 0, height: -10), blur: 20,
    color: CGColor(gray: 0, alpha: 0.3))
context.addPath(shape)
context.setFillColor(CGColor(gray: 0, alpha: 1))
context.fillPath()
context.restoreGState()

context.addPath(shape)
context.clip()
context.interpolationQuality = .high
context.draw(logo, in: rect)

let output = NSBitmapImageRep(cgImage: context.makeImage()!)
try output.representation(using: .png, properties: [:])!
    .write(to: URL(fileURLWithPath: arguments[2]))
