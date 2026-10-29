import 'api_error.dart';

/// Decides whether a request should be served from demo fixtures.
///
/// Demo data is used when the build forces demo mode, or when the real API is
/// unreachable (network failure, timeout, or gateway error). Real application
/// errors (400, 401, 403, 404, 409, 422, 429, 500) are never hidden behind demo
/// data: they surface as errors.
bool shouldUseDemo({required bool forceDemo, Object? error}) {
  if (forceDemo) return true;
  if (error is ApiException) return error.isUnreachable;
  return false;
}
