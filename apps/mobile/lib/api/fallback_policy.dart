/// Decides whether a request should be served from demo fixtures.
///
/// Only a build that forces demo mode (TRACE_DEMO=true) serves demo data. When the real
/// API is unreachable the app says so instead of showing fixtures: what people see is
/// always real.
bool shouldUseDemo({required bool forceDemo, Object? error}) => forceDemo;
