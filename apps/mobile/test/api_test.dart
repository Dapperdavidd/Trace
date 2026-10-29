import 'dart:convert';
import 'dart:io' show SocketException;

import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:trace_mobile/api/api_client.dart';
import 'package:trace_mobile/api/api_error.dart';
import 'package:trace_mobile/api/demo_api.dart';
import 'package:trace_mobile/api/fallback_policy.dart';
import 'package:trace_mobile/api/models.dart';
import 'package:trace_mobile/state/fallback_api.dart';

http.Response _json(Object body, [int status = 200]) =>
    http.Response(jsonEncode(body), status, headers: {'content-type': 'application/json'});

void main() {
  group('error envelope parsing', () {
    test('parses contract envelope', () {
      final e = ApiException.fromHttp(
          404, '{"code":"not_found","message":"No such wallet","details":{"x":1},"request_id":"r1"}');
      expect(e.code, 'not_found');
      expect(e.message, 'No such wallet');
      expect(e.requestId, 'r1');
      expect(e.status, 404);
      expect(e.details, isNotNull);
      expect(e.isUnreachable, isFalse);
    });
    test('non JSON and empty bodies fall back to generic error', () {
      final html = ApiException.fromHttp(500, '<html>oops</html>');
      expect(html.code, 'http_500');
      expect(html.message, contains('500'));
      expect(ApiException.fromHttp(400, '').code, 'http_400');
      expect(ApiException.fromHttp(400, '[1,2]').code, 'http_400');
    });
    test('unreachable classification', () {
      expect(ApiException.network().isUnreachable, isTrue);
      expect(ApiException.timeout().isUnreachable, isTrue);
      expect(ApiException.fromHttp(503, '').isUnreachable, isTrue);
      expect(ApiException.fromHttp(502, '').isUnreachable, isTrue);
      expect(ApiException.fromHttp(500, '').isUnreachable, isFalse);
      expect(ApiException.fromHttp(401, '').isUnauthorized, isTrue);
      expect(ApiException.parse().isUnreachable, isFalse);
    });
  });

  group('demo fallback decision', () {
    test('forced demo always uses demo', () {
      expect(shouldUseDemo(forceDemo: true), isTrue);
      expect(shouldUseDemo(forceDemo: true, error: ApiException.fromHttp(401, '')), isTrue);
    });
    test('network, timeout and gateway errors fall back', () {
      expect(shouldUseDemo(forceDemo: false, error: ApiException.network()), isTrue);
      expect(shouldUseDemo(forceDemo: false, error: ApiException.timeout()), isTrue);
      expect(shouldUseDemo(forceDemo: false, error: ApiException.fromHttp(503, '')), isTrue);
    });
    test('real application errors are not hidden by demo data', () {
      for (final s in [400, 401, 403, 404, 409, 422, 429, 500]) {
        expect(shouldUseDemo(forceDemo: false, error: ApiException.fromHttp(s, '')), isFalse, reason: '$s');
      }
      expect(shouldUseDemo(forceDemo: false, error: ApiException.parse()), isFalse);
      expect(shouldUseDemo(forceDemo: false), isFalse);
      expect(shouldUseDemo(forceDemo: false, error: StateError('x')), isFalse);
    });
  });

  group('ApiClient', () {
    test('builds /v1 URLs, drops empty query values, sends bearer token', () async {
      late http.Request seen;
      final c = ApiClient(
        baseUrl: 'http://host:8080/',
        client: MockClient((req) async {
          seen = req;
          return _json({'data_status': 'live', 'items': []});
        }),
      )..accessToken = 'tok';
      final r = await c.movers(tab: 'new', window: '1h');
      expect(r.env.status, DataStatus.live);
      expect(seen.url.path, '/v1/discovery/movers');
      expect(seen.url.queryParameters, {'tab': 'new', 'window': '1h'});
      expect(seen.headers['Authorization'], 'Bearer tok');
    });
    test('keeps a base path prefix and encodes wallet paths', () async {
      late Uri seen;
      final c = ApiClient(
        baseUrl: 'https://example.test/api',
        client: MockClient((req) async {
          seen = req.url;
          return _json({'data_status': 'live', 'wallet': {}, 'metrics': {}});
        }),
      );
      await c.wallet(Chain.base, '0xAbC');
      expect(seen.path, '/api/v1/wallets/base/0xAbC');
    });
    test('compare uses chain:address query', () async {
      late Uri seen;
      final c = ApiClient(
        baseUrl: 'http://h',
        client: MockClient((req) async {
          seen = req.url;
          return _json({'data_status': 'live', 'items': [], 'verdict_note': ''});
        }),
      );
      await c.compare(const TokenRef(chain: Chain.solana, address: 'A'), const TokenRef(chain: Chain.base, address: '0xB'));
      expect(seen.queryParameters, {'a': 'solana:A', 'b': 'base:0xB'});
    });
    test('maps HTTP error envelope', () async {
      final c = ApiClient(
        baseUrl: 'http://h',
        client: MockClient((req) async => _json({'code': 'rate_limited', 'message': 'Slow down', 'request_id': 'q'}, 429)),
      );
      await expectLater(
        c.token(Chain.solana, 'A'),
        throwsA(isA<ApiException>().having((e) => e.code, 'code', 'rate_limited').having((e) => e.requestId, 'request id', 'q')),
      );
    });
    test('maps socket failure to a network error', () async {
      final c = ApiClient(baseUrl: 'http://h', client: MockClient((req) async => throw const SocketException('no route')));
      await expectLater(c.briefings(), throwsA(isA<ApiException>().having((e) => e.kind, 'kind', ApiFailureKind.network)));
    });
    test('maps slow server to a timeout', () async {
      final c = ApiClient(
        baseUrl: 'http://h',
        timeout: const Duration(milliseconds: 20),
        client: MockClient((req) async {
          await Future<void>.delayed(const Duration(milliseconds: 200));
          return _json({});
        }),
      );
      await expectLater(c.briefings(), throwsA(isA<ApiException>().having((e) => e.kind, 'kind', ApiFailureKind.timeout)));
    });
    test('invalid JSON on 200 is a parse error', () async {
      final c = ApiClient(baseUrl: 'http://h', client: MockClient((req) async => http.Response('not json', 200)));
      await expectLater(c.briefings(), throwsA(isA<ApiException>().having((e) => e.kind, 'kind', ApiFailureKind.parse)));
    });
    test('createShadow posts the contract body and returns the id', () async {
      late http.Request seen;
      final c = ApiClient(
        baseUrl: 'http://h',
        client: MockClient((req) async {
          seen = req;
          return _json({'id': 's1'});
        }),
      );
      final id = await c.createShadow(const ShadowRequest(
        chain: Chain.solana,
        address: 'A',
        budgetUsd: '100',
        sizeRule: 'fixed_usd',
        delaySeconds: 30,
        slippageBps: 100,
        exitRule: 'mirror_sells',
      ));
      expect(id, 's1');
      expect(seen.method, 'POST');
      expect(seen.url.path, '/v1/shadow-strategies');
      expect(jsonDecode(seen.body), containsPair('budget_usd', '100'));
    });
    test('createShadow without an id is a parse error', () async {
      final c = ApiClient(baseUrl: 'http://h', client: MockClient((req) async => _json({})));
      await expectLater(
        c.createShadow(const ShadowRequest(
          chain: Chain.solana,
          address: 'A',
          budgetUsd: '1',
          sizeRule: 'x',
          delaySeconds: 0,
          slippageBps: 0,
          exitRule: 'y',
        )),
        throwsA(isA<ApiException>()),
      );
    });
    test('addWatch sends chain, address and optional alias', () async {
      late http.Request seen;
      final c = ApiClient(
        baseUrl: 'http://h',
        client: MockClient((req) async {
          seen = req;
          return http.Response('', 204);
        }),
      );
      await c.addWatch(Chain.solana, 'A', alias: '');
      expect(jsonDecode(seen.body), {'chain': 'solana', 'address': 'A'});
    });
  });

  group('FallbackApi', () {
    ApiClient failing(Object error) =>
        ApiClient(baseUrl: 'http://h', client: MockClient((req) async => throw error));

    test('serves flagged demo data when the server is unreachable', () async {
      final active = ValueNotifier<bool>(false);
      final api = FallbackApi(
        remote: failing(const SocketException('down')),
        demo: DemoApi(),
        forceDemo: false,
        demoActive: active,
      );
      final r = await api.movers();
      expect(r.env.isDemo, isTrue);
      expect(r.items, isNotEmpty);
      expect(active.value, isTrue);
    });
    test('does not hide real server errors behind demo data', () async {
      final active = ValueNotifier<bool>(false);
      final api = FallbackApi(
        remote: ApiClient(baseUrl: 'http://h', client: MockClient((req) async => _json({'code': 'forbidden', 'message': 'no'}, 403))),
        demo: DemoApi(),
        forceDemo: false,
        demoActive: active,
      );
      await expectLater(api.movers(), throwsA(isA<ApiException>()));
      expect(active.value, isFalse);
    });
    test('forced demo never touches the network and starts flagged', () async {
      var calls = 0;
      final active = ValueNotifier<bool>(false);
      final api = FallbackApi(
        remote: ApiClient(baseUrl: 'http://h', client: MockClient((req) async {
          calls++;
          return _json({});
        })),
        demo: DemoApi(),
        forceDemo: true,
        demoActive: active,
      );
      expect(active.value, isTrue);
      final t = await api.token(Chain.solana, DemoApi.addressOf('NEURA'));
      expect(t.env.isDemo, isTrue);
      expect(calls, 0);
    });
    test('real success clears the demo flag', () async {
      final active = ValueNotifier<bool>(false);
      final api = FallbackApi(
        remote: ApiClient(baseUrl: 'http://h', client: MockClient((req) async => _json({'data_status': 'live', 'sections': []}))),
        demo: DemoApi(),
        forceDemo: false,
        demoActive: active,
      );
      active.value = true; // pretend an earlier call fell back to demo
      final b = await api.briefings();
      expect(b.env.status, DataStatus.live);
      expect(active.value, isFalse);
    });
  });
}
