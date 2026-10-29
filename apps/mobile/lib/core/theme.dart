import 'package:flutter/material.dart';

import 'tokens.dart';

ThemeData buildTraceTheme() {
  const scheme = ColorScheme.dark(
    primary: TraceColors.emerald,
    onPrimary: TraceColors.onEmerald,
    secondary: TraceColors.emeraldDeep,
    onSecondary: TraceColors.text,
    surface: TraceColors.surface,
    onSurface: TraceColors.text,
    error: TraceColors.negative,
    onError: TraceColors.onEmerald,
    outline: TraceColors.border,
  );
  final base = ThemeData(
    useMaterial3: true,
    brightness: Brightness.dark,
    colorScheme: scheme,
    scaffoldBackgroundColor: TraceColors.bg,
    materialTapTargetSize: MaterialTapTargetSize.padded,
    visualDensity: VisualDensity.standard,
  );
  const minSize = Size(64, TraceSpacing.minTouch);
  final shape = RoundedRectangleBorder(borderRadius: BorderRadius.circular(14));
  return base.copyWith(
    textTheme: base.textTheme.apply(
      bodyColor: TraceColors.text,
      displayColor: TraceColors.text,
    ),
    appBarTheme: const AppBarTheme(
      backgroundColor: TraceColors.bg,
      foregroundColor: TraceColors.text,
      elevation: 0,
      scrolledUnderElevation: 0,
      centerTitle: false,
    ),
    navigationBarTheme: NavigationBarThemeData(
      backgroundColor: TraceColors.surface,
      indicatorColor: tint(TraceColors.emerald, 0.2),
      labelTextStyle: WidgetStateProperty.all(
        const TextStyle(fontSize: 12, color: TraceColors.text),
      ),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        backgroundColor: TraceColors.emerald,
        foregroundColor: TraceColors.onEmerald,
        disabledBackgroundColor: TraceColors.surfaceRaised,
        disabledForegroundColor: TraceColors.textMuted,
        minimumSize: minSize,
        shape: shape,
        textStyle: const TextStyle(fontWeight: FontWeight.w700, fontSize: 16),
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        foregroundColor: TraceColors.text,
        disabledForegroundColor: TraceColors.textMuted,
        minimumSize: minSize,
        shape: shape,
        side: const BorderSide(color: TraceColors.border),
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(
        foregroundColor: TraceColors.emerald,
        minimumSize: minSize,
      ),
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: TraceColors.surface,
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(12),
        borderSide: const BorderSide(color: TraceColors.border),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(12),
        borderSide: const BorderSide(color: TraceColors.border),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(12),
        borderSide: const BorderSide(color: TraceColors.emerald, width: 2),
      ),
      labelStyle: const TextStyle(color: TraceColors.textMuted),
    ),
    dividerColor: TraceColors.border,
    switchTheme: SwitchThemeData(
      thumbColor: WidgetStateProperty.resolveWith(
        (s) => s.contains(WidgetState.selected) ? TraceColors.onEmerald : TraceColors.textMuted,
      ),
      trackColor: WidgetStateProperty.resolveWith(
        (s) => s.contains(WidgetState.selected) ? TraceColors.emerald : TraceColors.surfaceRaised,
      ),
    ),
  );
}
