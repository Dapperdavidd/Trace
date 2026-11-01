import 'dart:convert';

import '../core/json_utils.dart';

enum ApiFailureKind { network, timeout, http, parse }

/// Error raised by the API layer. Mirrors the contract error envelope
/// `{ code, message, details?, request_id }`.
class ApiException implements Exception {
  const ApiException({
    required this.kind,
    required this.code,
    required this.message,
    this.status,
    this.requestId,
    this.details,
  });

  final ApiFailureKind kind;
  final String code;
  final String message;
  final int? status;
  final String? requestId;
  final Object? details;

  /// True when the server could not be reached or answered with a gateway error.
  bool get isUnreachable =>
      kind == ApiFailureKind.network ||
      kind == ApiFailureKind.timeout ||
      (kind == ApiFailureKind.http &&
          (status == 502 || status == 503 || status == 504));

  bool get isUnauthorized => status == 401;

  factory ApiException.network([String? detail]) => ApiException(
        kind: ApiFailureKind.network,
        code: 'network_unreachable',
        message: detail ?? 'Could not reach the GiFTED! server.',
      );

  factory ApiException.timeout() => const ApiException(
        kind: ApiFailureKind.timeout,
        code: 'timeout',
        message: 'The GiFTED! server took too long to answer.',
      );

  factory ApiException.parse([String? detail]) => ApiException(
        kind: ApiFailureKind.parse,
        code: 'bad_response',
        message:
            detail ?? 'The GiFTED! server sent a response we could not read.',
      );

  /// Parses an HTTP error response. Falls back to a generic error when the body
  /// is empty, not JSON, or not shaped like the contract envelope.
  factory ApiException.fromHttp(int status, String body) {
    Map<String, dynamic> m = const {};
    try {
      if (body.trim().isNotEmpty) m = asMap(jsonDecode(body));
    } on FormatException {
      m = const {};
    }
    final code = strOrNull(m['code']);
    final message = strOrNull(m['message']);
    return ApiException(
      kind: ApiFailureKind.http,
      status: status,
      code: code ?? 'http_$status',
      message: message ?? 'The request failed (HTTP $status).',
      requestId: strOrNull(m['request_id']),
      details: m['details'],
    );
  }

  @override
  String toString() => 'ApiException($code, $message, status=$status)';
}
