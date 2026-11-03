import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

const _clientOperations = <(String, String)>[
  ('/v1/auth/google', 'post'),
  ('/v1/auth/logout', 'post'),
  ('/v1/auth/logout-all', 'post'),
  ('/v1/auth/sessions', 'get'),
  ('/v1/auth/sessions/{id}', 'delete'),
  ('/v1/me', 'get'),
  ('/v1/me', 'delete'),
  ('/v1/watchlists', 'get'),
  ('/v1/watchlists', 'post'),
  ('/v1/watchlists/{id}', 'delete'),
  ('/v1/alerts', 'get'),
  ('/v1/discovery/movers', 'get'),
  ('/v1/discovery/wallets', 'get'),
  ('/v1/wallets/{chain}/{address}', 'get'),
  ('/v1/wallets/{chain}/{address}/trades', 'get'),
  ('/v1/wallets/{chain}/{address}/metrics', 'get'),
  ('/v1/wallets/{chain}/{address}/dna', 'get'),
  ('/v1/tokens/{chain}/{address}', 'get'),
  ('/v1/tokens/{chain}/{address}/timeline', 'get'),
  ('/v1/tokens/compare', 'get'),
  ('/v1/shadow-strategies', 'post'),
  ('/v1/shadow-strategies/{id}/results', 'get'),
  ('/v1/briefings', 'get'),
  ('/v1/chains', 'get'),
];

File _snapshot() {
  for (final path in [
    '../../docs/openapi.snapshot.json',
    'docs/openapi.snapshot.json',
  ]) {
    final file = File(path);
    if (file.existsSync()) return file;
  }
  throw StateError('Could not find docs/openapi.snapshot.json.');
}

void main() {
  test('OpenAPI contains every endpoint and method used by the mobile client',
      () {
    final document =
        jsonDecode(_snapshot().readAsStringSync()) as Map<String, dynamic>;
    final paths = document['paths'] as Map<String, dynamic>;
    for (final (path, method) in _clientOperations) {
      expect(paths, contains(path), reason: 'missing $path');
      expect(paths[path] as Map<String, dynamic>, contains(method),
          reason: 'missing ${method.toUpperCase()} $path');
    }
  });
}
