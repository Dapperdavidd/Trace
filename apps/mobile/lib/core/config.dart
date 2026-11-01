/// Build-time configuration, supplied with --dart-define.
class AppConfig {
  const AppConfig._();

  static const String apiUrl = String.fromEnvironment('TRACE_API_URL',
      defaultValue: 'http://10.0.2.2:8080');

  /// When true the app never calls the network and serves demo fixtures.
  static const bool demoForced =
      bool.fromEnvironment('TRACE_DEMO', defaultValue: false);

  static const String googleClientId =
      String.fromEnvironment('TRACE_GOOGLE_CLIENT_ID');
  static const String googleServerClientId =
      String.fromEnvironment('TRACE_GOOGLE_SERVER_CLIENT_ID');

  /// Enables the explicit local-only development sign-in surface.
  static const bool devAuth =
      bool.fromEnvironment('TRACE_DEV_AUTH', defaultValue: false);

  static bool get googleConfigured =>
      googleClientId.isNotEmpty || googleServerClientId.isNotEmpty;

  static const int alertPollSeconds = 30;
  static const Duration requestTimeout = Duration(seconds: 12);
}
