import 'package:flutter/material.dart';

import 'tokens.dart';

/// Material theme in the trace v11 language: Geist, pill buttons, filled panels, no borders.
ThemeData buildTraceTheme() {
  const scheme = ColorScheme.dark(
    primary: TraceColors.emerald,
    onPrimary: TraceColors.onEmerald,
    secondary: TraceColors.emerald,
    onSecondary: TraceColors.onEmerald,
    surface: TraceColors.surface,
    onSurface: TraceColors.text,
    surfaceContainerHighest: TraceColors.surfaceRaised,
    error: TraceColors.text,
    onError: TraceColors.bg,
    outline: TraceColors.border2,
    outlineVariant: TraceColors.border,
  );
  final base = ThemeData(
    useMaterial3: true,
    brightness: Brightness.dark,
    colorScheme: scheme,
    fontFamily: traceFont,
    scaffoldBackgroundColor: TraceColors.bg,
    canvasColor: TraceColors.bg,
    materialTapTargetSize: MaterialTapTargetSize.padded,
    visualDensity: VisualDensity.standard,
    splashFactory: InkSparkle.splashFactory,
  );
  const pill = StadiumBorder();
  const minSize = Size(64, TraceSpacing.minTouch);
  final text = base.textTheme
      .apply(
        fontFamily: traceFont,
        bodyColor: TraceColors.text,
        displayColor: TraceColors.text,
      )
      .copyWith(
        displaySmall: const TextStyle(
            fontSize: 34,
            fontWeight: FontWeight.w500,
            letterSpacing: -1.4,
            height: 1.05),
        headlineMedium: const TextStyle(
            fontSize: 28,
            fontWeight: FontWeight.w500,
            letterSpacing: -1.1,
            height: 1.1),
        headlineSmall: const TextStyle(
            fontSize: 24, fontWeight: FontWeight.w500, letterSpacing: -0.8),
        titleLarge: const TextStyle(
            fontSize: 20, fontWeight: FontWeight.w500, letterSpacing: -0.5),
        titleMedium: const TextStyle(
            fontSize: 16, fontWeight: FontWeight.w500, letterSpacing: -0.2),
        bodyMedium: const TextStyle(fontSize: 14.5, height: 1.45),
        bodySmall: const TextStyle(
            fontSize: 13, color: TraceColors.textFaint, height: 1.4),
        labelLarge:
            const TextStyle(fontSize: 15, fontWeight: FontWeight.w500),
      );
  return base.copyWith(
    textTheme: text,
    appBarTheme: const AppBarTheme(
      backgroundColor: TraceColors.bg,
      foregroundColor: TraceColors.text,
      elevation: 0,
      scrolledUnderElevation: 0,
      centerTitle: false,
      titleTextStyle: TextStyle(
          fontFamily: traceFont,
          fontSize: 24,
          fontWeight: FontWeight.w500,
          letterSpacing: -0.8,
          color: TraceColors.text),
    ),
    cardTheme: CardThemeData(
      color: TraceColors.surface,
      elevation: 0,
      margin: EdgeInsets.zero,
      shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(TraceSpacing.radius)),
    ),
    chipTheme: base.chipTheme.copyWith(
      backgroundColor: TraceColors.surfaceRaised,
      selectedColor: TraceColors.emerald,
      side: BorderSide.none,
      shape: pill,
      labelStyle: const TextStyle(
          fontFamily: traceFont, fontSize: 13.5, color: TraceColors.textMuted),
      secondaryLabelStyle: const TextStyle(
          fontFamily: traceFont,
          fontSize: 13.5,
          color: TraceColors.onEmerald,
          fontWeight: FontWeight.w500),
    ),
    segmentedButtonTheme: SegmentedButtonThemeData(
      style: ButtonStyle(
        shape: const WidgetStatePropertyAll(pill),
        side: const WidgetStatePropertyAll(BorderSide.none),
        backgroundColor: WidgetStateProperty.resolveWith((s) =>
            s.contains(WidgetState.selected)
                ? TraceColors.emerald
                : TraceColors.surfaceRaised),
        foregroundColor: WidgetStateProperty.resolveWith((s) =>
            s.contains(WidgetState.selected)
                ? TraceColors.onEmerald
                : TraceColors.textMuted),
      ),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        backgroundColor: TraceColors.emerald,
        foregroundColor: TraceColors.onEmerald,
        disabledBackgroundColor: TraceColors.surfaceRaised,
        disabledForegroundColor: TraceColors.textFaint,
        minimumSize: minSize,
        shape: pill,
        padding: const EdgeInsets.symmetric(horizontal: 22, vertical: 14),
        textStyle: const TextStyle(
            fontFamily: traceFont, fontWeight: FontWeight.w600, fontSize: 15),
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        backgroundColor: TraceColors.surfaceRaised,
        foregroundColor: TraceColors.text,
        disabledForegroundColor: TraceColors.textFaint,
        minimumSize: minSize,
        shape: pill,
        side: BorderSide.none,
        padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 14),
        textStyle: const TextStyle(
            fontFamily: traceFont, fontWeight: FontWeight.w500, fontSize: 15),
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(
        foregroundColor: TraceColors.emerald,
        minimumSize: minSize,
        shape: pill,
        textStyle: const TextStyle(
            fontFamily: traceFont, fontWeight: FontWeight.w500, fontSize: 14),
      ),
    ),
    iconButtonTheme: IconButtonThemeData(
      style: IconButton.styleFrom(
        backgroundColor: TraceColors.surfaceRaised,
        foregroundColor: TraceColors.text,
      ),
    ),
    listTileTheme: const ListTileThemeData(
      iconColor: TraceColors.textMuted,
      textColor: TraceColors.text,
      contentPadding: EdgeInsets.symmetric(horizontal: 4),
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: TraceColors.surface,
      hintStyle: const TextStyle(color: TraceColors.textFaint),
      labelStyle: const TextStyle(color: TraceColors.textFaint),
      contentPadding: const EdgeInsets.symmetric(horizontal: 18, vertical: 14),
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(99),
        borderSide: BorderSide.none,
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(99),
        borderSide: BorderSide.none,
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(99),
        borderSide: const BorderSide(color: TraceColors.emerald, width: 1.5),
      ),
    ),
    dividerTheme:
        const DividerThemeData(color: TraceColors.border, thickness: 1),
    dividerColor: TraceColors.border,
    snackBarTheme: const SnackBarThemeData(
      backgroundColor: TraceColors.emerald,
      contentTextStyle: TextStyle(
          fontFamily: traceFont,
          color: TraceColors.onEmerald,
          fontWeight: FontWeight.w500),
      behavior: SnackBarBehavior.floating,
      shape: StadiumBorder(),
    ),
    progressIndicatorTheme:
        const ProgressIndicatorThemeData(color: TraceColors.emerald),
    switchTheme: SwitchThemeData(
      trackOutlineColor: const WidgetStatePropertyAll(Colors.transparent),
      thumbColor: WidgetStateProperty.resolveWith((s) =>
          s.contains(WidgetState.selected)
              ? TraceColors.onEmerald
              : TraceColors.textMuted),
      trackColor: WidgetStateProperty.resolveWith((s) =>
          s.contains(WidgetState.selected)
              ? TraceColors.emerald
              : TraceColors.surfaceHigh),
    ),
  );
}
