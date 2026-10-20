// qrdecode: prints the text of every QR code in the given images, one per line, read with Apple's
// Vision framework (macOS only). tests/share_check.sh builds it with swiftc to check that the Share
// screen's code really reads back as its address:
//   swiftc -O tools/qrdecode.swift -o build/host/qrdecode
//   build/host/qrdecode [--polarity] build/shots/c_share/01-share.png
// --polarity adds how each code is drawn: "dark-on-light" (an ordinary code) or "light-on-dark"
// (its negative; Vision reads both, some phone cameras only the first).
// Exit status: 0 when every image held a code, 1 when one held none, 2 when one could not be read.
import CoreGraphics
import CoreImage
import Foundation
import ImageIO
import Vision

func fail(_ message: String) {
    FileHandle.standardError.write(Data("qrdecode: \(message)\n".utf8))
}

// The image as 8-bit gray, top row first.
func grayPixels(_ image: CGImage) -> [UInt8]? {
    let w = image.width, h = image.height
    var px = [UInt8](repeating: 0, count: w * h)
    let drawn = px.withUnsafeMutableBytes { buf -> Bool in
        guard let ctx = CGContext(data: buf.baseAddress, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w,
                                  space: CGColorSpaceCreateDeviceGray(), bitmapInfo: CGImageAlphaInfo.none.rawValue)
        else { return false }
        ctx.draw(image, in: CGRect(x: 0, y: 0, width: w, height: h))
        return true
    }
    return drawn ? px : nil
}

// The middle of a finder pattern (the squares in three corners of a code, 7 modules across) is
// dark in an ordinary code. Looks 3.5 modules in from each of the four corners: three of them are
// finder patterns whichever way the code is turned, the fourth is data.
func polarity(_ code: VNBarcodeObservation, _ px: [UInt8], _ w: Int, _ h: Int) -> String {
    guard let qr = code.barcodeDescriptor as? CIQRCodeDescriptor else { return "unknown" }
    let f = 3.5 / CGFloat(17 + 4 * qr.symbolVersion)
    let tl = code.topLeft, tr = code.topRight, bl = code.bottomLeft, br = code.bottomRight
    var dark = 0
    for (c, a, b) in [(tl, tr, bl), (tr, tl, br), (bl, br, tl), (br, bl, tr)] {
        let x = c.x + (a.x - c.x) * f + (b.x - c.x) * f, y = c.y + (a.y - c.y) * f + (b.y - c.y) * f
        let ix = min(max(Int(x * CGFloat(w)), 0), w - 1), iy = min(max(Int((1 - y) * CGFloat(h)), 0), h - 1)
        if px[iy * w + ix] < 128 { dark += 1 }  // Vision's points: 0..1, origin bottom left
    }
    return dark >= 3 ? "dark-on-light" : dark <= 1 ? "light-on-dark" : "unknown"
}

var withPolarity = false
var status: Int32 = 0
for path in CommandLine.arguments.dropFirst() {
    if path == "--polarity" {
        withPolarity = true
        continue
    }
    guard let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: path) as CFURL, nil),
          let image = CGImageSourceCreateImageAtIndex(source, 0, nil)
    else {
        fail("cannot read \(path)")
        status = 2
        continue
    }
    let request = VNDetectBarcodesRequest()
    request.symbologies = [.qr]
    do {
        try VNImageRequestHandler(cgImage: image, options: [:]).perform([request])
    } catch {
        fail("\(path): \(error)")
        status = 2
        continue
    }
    let codes = (request.results ?? []).filter { $0.payloadStringValue != nil }
    if codes.isEmpty {
        fail("no QR code in \(path)")
        status = max(status, 1)
    }
    let px = withPolarity ? grayPixels(image) : nil
    for code in codes {
        let text = code.payloadStringValue ?? ""
        if let px = px { print("\(text) \(polarity(code, px, image.width, image.height))") } else { print(text) }
    }
}
exit(status)
