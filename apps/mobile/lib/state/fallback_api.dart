import 'package:flutter/foundation.dart';

import '../api/api_error.dart';
import '../api/demo_api.dart';
import '../api/fallback_policy.dart';
import '../api/models.dart';
import '../api/trace_api.dart';

/// Wraps the real API. Serves demo fixtures when demo is forced or when the
/// server is unreachable, and reports the current mode through [demoActive] so
/// the UI can show the DEMO DATA banner on every screen.
class FallbackApi implements TraceApi {
  FallbackApi({
    required this.remote,
    required this.demo,
    required this.forceDemo,
    required this.demoActive,
  }) {
    demoActive.value = forceDemo;
  }

  final TraceApi remote;
  final DemoApi demo;
  final bool forceDemo;
  final ValueNotifier<bool> demoActive;

  Future<T> _run<T>(Future<T> Function(TraceApi api) call) async {
    if (shouldUseDemo(forceDemo: forceDemo)) {
      final r = await call(demo);
      demoActive.value = true;
      return r;
    }
    try {
      final r = await call(remote);
      demoActive.value = false;
      return r;
    } on ApiException catch (e) {
      if (shouldUseDemo(forceDemo: forceDemo, error: e)) {
        final r = await call(demo);
        demoActive.value = true;
        return r;
      }
      rethrow;
    }
  }

  // Auth never falls back to demo: a demo session would be misleading.
  @override
  Future<AuthResult> googleSignIn(String idToken) => remote.googleSignIn(idToken);

  @override
  Future<void> logout() async {
    if (forceDemo) return;
    try {
      await remote.logout();
    } on ApiException {
      // Local sign-out proceeds even if the server cannot be reached.
    }
  }

  @override
  Future<void> deleteAccount() => forceDemo ? demo.deleteAccount() : remote.deleteAccount();

  @override
  Future<Me> me() => _run((a) => a.me());

  @override
  Future<List<WatchlistItem>> watchlists() => _run((a) => a.watchlists());

  @override
  Future<void> addWatch(Chain chain, String address, {String? alias}) =>
      _run((a) => a.addWatch(chain, address, alias: alias));

  @override
  Future<void> removeWatch(String id) => _run((a) => a.removeWatch(id));

  @override
  Future<AlertsPage> alerts({String? cursor}) => _run((a) => a.alerts(cursor: cursor));

  @override
  Future<MoversResponse> movers({String tab = 'trending', String window = '24h', Chain? chain}) =>
      _run((a) => a.movers(tab: tab, window: window, chain: chain));

  @override
  Future<WalletsResponse> discoveryWallets({String tab = 'top', Chain? chain, String? cursor}) =>
      _run((a) => a.discoveryWallets(tab: tab, chain: chain, cursor: cursor));

  @override
  Future<WalletDetail> wallet(Chain chain, String address) =>
      _run((a) => a.wallet(chain, address));

  @override
  Future<TradesResponse> walletTrades(Chain chain, String address, {String? cursor}) =>
      _run((a) => a.walletTrades(chain, address, cursor: cursor));

  @override
  Future<WalletMetricsResponse> walletMetrics(Chain chain, String address) =>
      _run((a) => a.walletMetrics(chain, address));

  @override
  Future<DnaResponse> walletDna(Chain chain, String address) =>
      _run((a) => a.walletDna(chain, address));

  @override
  Future<TokenDetail> token(Chain chain, String address) =>
      _run((a) => a.token(chain, address));

  @override
  Future<TimelineResponse> tokenTimeline(Chain chain, String address) =>
      _run((a) => a.tokenTimeline(chain, address));

  @override
  Future<CompareResponse> compare(TokenRef a, TokenRef b) =>
      _run((api) => api.compare(a, b));

  @override
  Future<String> createShadow(ShadowRequest request) =>
      _run((a) => a.createShadow(request));

  @override
  Future<ShadowResults> shadowResults(String id) {
    // Demo ids always resolve to demo results, even if the server came back.
    if (id.startsWith('demo-')) {
      return demo.shadowResults(id).then((r) {
        demoActive.value = true;
        return r;
      });
    }
    return _run((a) => a.shadowResults(id));
  }

  @override
  Future<BriefingResponse> briefings() => _run((a) => a.briefings());
}
