import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../core/tokens.dart';

/// The trace block field from the v11 design: an isometric grid of blocks, one lifted
/// and lit green (the block the lens opened). Port of the web's static `isoField`.
class BlockField extends StatefulWidget {
  const BlockField({super.key});

  @override
  State<BlockField> createState() => _BlockFieldState();
}

class _BlockFieldState extends State<BlockField>
    with SingleTickerProviderStateMixin {
  late final AnimationController _c = AnimationController(
      vsync: this, duration: const Duration(seconds: 6))
    ..repeat();

  @override
  void dispose() {
    _c.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final reduce = MediaQuery.of(context).disableAnimations;
    return ExcludeSemantics(
      child: AnimatedBuilder(
        animation: _c,
        builder: (_, __) => CustomPaint(
          painter: _FieldPainter(reduce ? 0.5 : _c.value),
          size: Size.infinite,
        ),
      ),
    );
  }
}

class _FieldPainter extends CustomPainter {
  _FieldPainter(this.t);
  final double t;

  @override
  void paint(Canvas canvas, Size size) {
    final s = math.min(size.width, size.height) / 19;
    final cx = size.width / 2;
    final cy = size.height * 0.56;
    Offset iso(double x, double y, double z) =>
        Offset(cx + (x - z) * 0.866 * s, cy + (x + z) * 0.5 * s - y * s);

    final rnd = math.Random(5);
    final blocks = <(double, double, double, bool)>[];
    for (var i = 0; i < 7; i++) {
      for (var j = 0; j < 7; j++) {
        blocks.add(((j - 3) * 2.4, (i - 3) * 2.7, 0.7 + rnd.nextDouble() * 0.6,
            i == 3 && j == 3));
      }
    }
    blocks.sort((a, b) => (a.$1 + a.$2).compareTo(b.$1 + b.$2));

    // The open block breathes up and down, like the lens opening it on the web.
    final lift = 1.0 + 0.4 * math.sin(t * 2 * math.pi);
    final edge = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1
      ..color = TraceColors.text.withAlpha(70);
    final edgeOn = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.2
      ..color = TraceColors.emerald;
    final fill = Paint()..color = const Color(0xFF131816);
    final fillOn = Paint()..color = TraceColors.emeraldDeep;

    for (final b in blocks) {
      final (x, z, h, key) = b;
      final y0 = key ? lift : 0.0;
      final y1 = y0 + h;
      const k = 0.65;
      final c = [
        iso(x - k, y0, z - k), iso(x + k, y0, z - k), iso(x + k, y0, z + k),
        iso(x - k, y0, z + k), iso(x - k, y1, z - k), iso(x + k, y1, z - k),
        iso(x + k, y1, z + k), iso(x - k, y1, z + k),
      ];
      for (final face in const [
        [1, 2, 6, 5],
        [2, 3, 7, 6],
        [4, 5, 6, 7],
      ]) {
        final path = Path()..moveTo(c[face[0]].dx, c[face[0]].dy);
        for (final i in face.skip(1)) {
          path.lineTo(c[i].dx, c[i].dy);
        }
        path.close();
        canvas.drawPath(path, key ? fillOn : fill);
        canvas.drawPath(path, key ? edgeOn : edge);
      }
      if (key) {
        // the lens hovering above the open block
        final top = iso(x, y1 + 2.2, z);
        final r = s * 1.15;
        canvas.drawCircle(top, r, Paint()..color = TraceColors.text.withAlpha(14));
        canvas.drawCircle(
            top,
            r,
            Paint()
              ..style = PaintingStyle.stroke
              ..strokeWidth = 3
              ..color = TraceColors.text);
        canvas.drawLine(
            top + Offset(r * 0.74, r * 0.74),
            top + Offset(r * 0.74 + s * 0.9, r * 0.74 + s * 0.9),
            Paint()
              ..strokeWidth = 5
              ..strokeCap = StrokeCap.round
              ..color = TraceColors.text);
        canvas.drawArc(
            Rect.fromCircle(center: top, radius: r + s * 0.28),
            -math.pi / 2,
            2 * math.pi * t,
            false,
            Paint()
              ..style = PaintingStyle.stroke
              ..strokeWidth = 2.5
              ..color = TraceColors.emerald);
      }
    }
  }

  @override
  bool shouldRepaint(_FieldPainter old) => old.t != t;
}
