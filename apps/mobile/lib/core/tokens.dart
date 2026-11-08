import 'dart:ui';

/// Design tokens from the trace v11 design (apps/web/design/trace-redesign-v11.html):
/// one dark theme, two flat colours (black and green) with white text tints. Soft and
/// rounded: filled surfaces instead of boxes, pills instead of square buttons.
class TraceColors {
  const TraceColors._();

  static const Color bg = Color(0xFF0A0B0B);
  static const Color surface = Color(0xFF131716); // --panel
  static const Color surfaceRaised = Color(0xFF1B201E); // --panel2
  static const Color surfaceHigh = Color(0xFF232927); // --panel3
  static const Color border = Color(0x12EDF2EF); // --line
  static const Color border2 = Color(0x1FEDF2EF); // --line2

  static const Color emerald = Color(0xFF4387FF); // primary blue
  static const Color emeraldSoft = Color(0x334387FF);
  static const Color emeraldDeep = Color(0xFF12264A);
  static const Color onEmerald = Color(0xFFFFFFFF);

  static const Color text = Color(0xFFEDF2EF); // --fg
  static const Color textMuted = Color(0xA8EDF2EF); // --fg2
  static const Color textFaint = Color(0x6BEDF2EF); // --fg3

  static const Color positive = emerald;

  /// The design never uses red: losses and warnings are shown in muted or inverted text.
  static const Color negative = textMuted;
  static const Color warning = text;
  static const Color info = textMuted;
}

class TraceSpacing {
  const TraceSpacing._();
  static const double gutter = 16;
  static const double gap = 12;
  static const double small = 8;
  static const double radius = 22; // --r-lg
  static const double radiusMd = 16;
  static const double minTouch = 48;
}

const String traceFont = 'Geist';

/// Same colour with a different opacity, without deprecated APIs.
Color tint(Color c, double opacity) =>
    c.withAlpha((opacity * 255).round().clamp(0, 255));
