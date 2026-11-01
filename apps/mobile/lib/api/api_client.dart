import 'dart:async';
import 'dart:convert';
import 'dart:io' show SocketException;

import 'package:http/http.dart' as http;

import '../core/config.dart';
import '../core/json_utils.dart';
import 'api_error.dart';
import 'models.dart';
import 'trace_api.dart';

/// HTTP implementation of the TRACE contract (base path /v1).
class ApiClient implements TraceApi {
  ApiClient({
    String? baseUrl,
    http.Client? client,
    Duration? timeout,
  })  : baseUrl = baseUrl ?? AppConfig.apiUrl,
        _client = client ?? http.Client(),
        timeout = timeout ?? AppConfig.requestTimeout;

  final String baseUrl;
  final Duration timeout;
  final http.Client _client;

  /// Session token, set by the session controller after Google sign-in.
  String? accessToken;

  Uri buildUri(String path, [Map<String, String?>? query]) {
    final base = Uri.parse(baseUrl);
    final prefix = base.path.replaceAll(RegExp(r'/+$'), '');
    final q = <String, String>{};
    query?.forEach((k, v) {
      if (v != null && v.isNotEmpty) q[k] = v;
    });
    return base.replace(
      path: '$prefix/v1$path',
      queryParameters: q.isEmpty ? null : q,
    );
  }

  Future<Object?> _send(
    String method,
    String path, {
    Map<String, String?>? query,
    Object? body,
  }) async {
    final uri = buildUri(path, query);
    final headers = <String, String>{
      'Accept': 'application/json',
      if (body != null) 'Content-Type': 'application/json',
      if (accessToken != null) 'Authorization': 'Bearer $accessToken',
    };
    try {
      final req = http.Request(method, uri)..headers.addAll(headers);
      if (body != null) req.body = jsonEncode(body);
      final streamed = await _client.send(req).timeout(timeout);
      final res = await http.Response.fromStream(streamed).timeout(timeout);
      if (res.statusCode >= 400) {
        throw ApiException.fromHttp(res.statusCode, res.body);
      }
      if (res.body.trim().isEmpty) return <String, dynamic>{};
      try {
        return jsonDecode(res.body);
      } on FormatException {
        throw ApiException.parse();
      }
    } on TimeoutException {
      throw ApiException.timeout();
    } on SocketException catch (e) {
      throw ApiException.network(e.message);
    } on http.ClientException catch (e) {
      throw ApiException.network(e.message);
    }
  }

  Future<Map<String, dynamic>> _getMap(String path,
          [Map<String, String?>? q]) async =>
      asMap(await _send('GET', path, query: q));

  String _w(Chain c, String a) => '/${c.wire}/${Uri.encodeComponent(a)}';

  // Auth and account ---------------------------------------------------------

  @override
  Future<AuthResult> googleSignIn(String idToken) async => AuthResult.fromJson(
      asMap(await _send('POST', '/auth/google', body: {'id_token': idToken})));

  @override
  Future<void> logout() async {
    await _send('POST', '/auth/logout');
  }

  @override
  Future<void> deleteAccount() async {
    await _send('DELETE', '/me');
  }

  @override
  Future<Me> me() async => Me.fromJson(await _getMap('/me'));

  // Watchlist and alerts -----------------------------------------------------

  @override
  Future<List<WatchlistItem>> watchlists() async =>
      mapList((await _getMap('/watchlists'))['items'], WatchlistItem.fromJson);

  @override
  Future<void> addWatch(Chain chain, String address, {String? alias}) async {
    await _send('POST', '/watchlists', body: {
      'chain': chain.wire,
      'address': address,
      if (alias != null && alias.isNotEmpty) 'alias': alias,
    });
  }

  @override
  Future<void> removeWatch(String id) async {
    await _send('DELETE', '/watchlists/${Uri.encodeComponent(id)}');
  }

  @override
  Future<AlertsPage> alerts({String? cursor}) async =>
      AlertsPage.fromJson(await _getMap('/alerts', {'cursor': cursor}));

  // Discovery ----------------------------------------------------------------

  @override
  Future<MoversResponse> movers({
    String tab = 'trending',
    String window = '24h',
    Chain? chain,
  }) async =>
      MoversResponse.fromJson(await _getMap('/discovery/movers', {
        'tab': tab,
        'window': window,
        'chain': chain?.wire,
      }));

  @override
  Future<WalletsResponse> discoveryWallets({
    String tab = 'top',
    Chain? chain,
    String? cursor,
  }) async =>
      WalletsResponse.fromJson(await _getMap('/discovery/wallets', {
        'tab': tab,
        'chain': chain?.wire,
        'cursor': cursor,
      }));

  // Wallet research ----------------------------------------------------------

  @override
  Future<WalletDetail> wallet(Chain chain, String address) async =>
      WalletDetail.fromJson(await _getMap('/wallets${_w(chain, address)}'));

  @override
  Future<TradesResponse> walletTrades(Chain chain, String address,
          {String? cursor}) async =>
      TradesResponse.fromJson(await _getMap(
          '/wallets${_w(chain, address)}/trades', {'cursor': cursor}));

  @override
  Future<WalletMetricsResponse> walletMetrics(
          Chain chain, String address) async =>
      WalletMetricsResponse.fromJson(
          await _getMap('/wallets${_w(chain, address)}/metrics'));

  @override
  Future<DnaResponse> walletDna(Chain chain, String address) async =>
      DnaResponse.fromJson(await _getMap('/wallets${_w(chain, address)}/dna'));

  // Tokens -------------------------------------------------------------------

  @override
  Future<TokenDetail> token(Chain chain, String address) async =>
      TokenDetail.fromJson(await _getMap('/tokens${_w(chain, address)}'));

  @override
  Future<TimelineResponse> tokenTimeline(Chain chain, String address) async =>
      TimelineResponse.fromJson(
          await _getMap('/tokens${_w(chain, address)}/timeline'));

  @override
  Future<CompareResponse> compare(TokenRef a, TokenRef b) async =>
      CompareResponse.fromJson(await _getMap('/tokens/compare', {
        'a': '${a.chain.wire}:${a.address}',
        'b': '${b.chain.wire}:${b.address}',
      }));

  // Shadow strategies --------------------------------------------------------

  @override
  Future<String> createShadow(ShadowRequest request) async {
    final m = asMap(
        await _send('POST', '/shadow-strategies', body: request.toJson()));
    final id = strOrNull(m['id']);
    if (id == null || id.isEmpty) {
      throw ApiException.parse('Missing strategy id.');
    }
    return id;
  }

  @override
  Future<ShadowResults> shadowResults(String id) async =>
      ShadowResults.fromJson(await _getMap(
          '/shadow-strategies/${Uri.encodeComponent(id)}/results'));

  // Briefings ----------------------------------------------------------------

  @override
  Future<BriefingResponse> briefings() async =>
      BriefingResponse.fromJson(await _getMap('/briefings'));

  @override
  Future<CoverageResponse> coverage() async =>
      CoverageResponse.fromJson(await _getMap('/chains'));
}
