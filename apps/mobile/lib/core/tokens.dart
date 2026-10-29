import 'dart:ui';

/// Design tokens: Emerald Green on Charcoal Black.
class TraceColors {
  const TraceColors._();

  static const Color bg = Color(0xFF090E0C);
  static const Color surface = Color(0xFF111915);
  static const Color surfaceRaised = Color(0xFF17211C);
  static const Color border = Color(0xFF24332B);

  static const Color emerald = Color(0xFF3DDC97);
  static const Color emeraldDeep = Color(0xFF1F9D6B);
  static const Color onEmerald = Color(0xFF04110B);

  static const Color text = Color(0xFFEAF5EF);
  static const Color textMuted = Color(0xFF9DB1A7);

  static const Color positive = Color(0xFF3DDC97);
  static const Color negative = Color(0xFFFF7A7A);
  static const Color warning = Color(0xFFF5C04A);
  static const Color info = Color(0xFF6DB3FF);
}

class TraceSpacing {
  const TraceSpacing._();
  static const double gutter = 16;
  static const double gap = 12;
  static const double small = 8;
  static const double radius = 16;
  static const double minTouch = 48;
}

/// Same colour with a different opacity, without deprecated APIs.
Color tint(Color c, double opacity) => c.withAlpha((opacity * 255).round().clamp(0, 255));
