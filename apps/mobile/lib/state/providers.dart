import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../api/api_client.dart';
import '../api/demo_api.dart';
import '../api/models.dart';
import '../api/trace_api.dart';
import '../core/config.dart';
import 'fallback_api.dart';

final apiClientProvider = Provider<ApiClient>((ref) => ApiClient());

final demoApiProvider = Provider<DemoApi>((ref) => DemoApi());

/// True while the app is showing demo fixtures. Drives the DEMO DATA banner.
final demoActiveProvider =
    Provider<ValueNotifier<bool>>((ref) => ValueNotifier<bool>(AppConfig.demoForced));

final apiProvider = Provider<TraceApi>((ref) => FallbackApi(
      remote: ref.watch(apiClientProvider),
      demo: ref.watch(demoApiProvider),
      forceDemo: AppConfig.demoForced,
      demoActive: ref.watch(demoActiveProvider),
    ));

/// (tab, window, chain wire or empty)
typedef MoversQuery = (String, String, String);

final moversProvider =
    FutureProvider.autoDispose.family<MoversResponse, MoversQuery>((ref, q) {
  return ref.watch(apiProvider).movers(
        tab: q.$1,
        window: q.$2,
        chain: q.$3.isEmpty ? null : Chain.parse(q.$3),
      );
});

final discoveryWalletsProvider =
    FutureProvider.autoDispose.family<WalletsResponse, String>((ref, tab) {
  return ref.watch(apiProvider).discoveryWallets(tab: tab);
});

typedef AssetKey = (Chain, String);

final tokenProvider = FutureProvider.autoDispose.family<TokenDetail, AssetKey>(
    (ref, k) => ref.watch(apiProvider).token(k.$1, k.$2));

final timelineProvider = FutureProvider.autoDispose.family<TimelineResponse, AssetKey>(
    (ref, k) => ref.watch(apiProvider).tokenTimeline(k.$1, k.$2));

final walletDetailProvider = FutureProvider.autoDispose.family<WalletDetail, AssetKey>(
    (ref, k) => ref.watch(apiProvider).wallet(k.$1, k.$2));

final walletTradesProvider = FutureProvider.autoDispose.family<TradesResponse, AssetKey>(
    (ref, k) => ref.watch(apiProvider).walletTrades(k.$1, k.$2));

final walletMetricsProvider =
    FutureProvider.autoDispose.family<WalletMetricsResponse, AssetKey>(
        (ref, k) => ref.watch(apiProvider).walletMetrics(k.$1, k.$2));

final walletDnaProvider = FutureProvider.autoDispose.family<DnaResponse, AssetKey>(
    (ref, k) => ref.watch(apiProvider).walletDna(k.$1, k.$2));

/// Compare key: (chainA, addrA, chainB, addrB)
typedef CompareKey = (Chain, String, Chain, String);

final compareProvider = FutureProvider.autoDispose.family<CompareResponse, CompareKey>(
    (ref, k) => ref.watch(apiProvider).compare(
          TokenRef(chain: k.$1, address: k.$2),
          TokenRef(chain: k.$3, address: k.$4),
        ));

final watchlistProvider = FutureProvider.autoDispose<List<WatchlistItem>>(
    (ref) => ref.watch(apiProvider).watchlists());

final briefingProvider = FutureProvider.autoDispose<BriefingResponse>(
    (ref) => ref.watch(apiProvider).briefings());

final shadowResultsProvider = FutureProvider.autoDispose.family<ShadowResults, String>(
    (ref, id) => ref.watch(apiProvider).shadowResults(id));

/// In-app alert polling. No push notifications in this version.
final alertsProvider = StreamProvider.autoDispose<AlertsPage>((ref) async* {
  final api = ref.watch(apiProvider);
  var alive = true;
  ref.onDispose(() => alive = false);
  while (alive) {
    try {
      yield await api.alerts();
    } catch (e) {
      yield* Stream<AlertsPage>.error(e);
    }
    await Future<void>.delayed(const Duration(seconds: AppConfig.alertPollSeconds));
  }
});

/// Ids of paper simulations started in this app session.
class ShadowIdsNotifier extends Notifier<List<String>> {
  @override
  List<String> build() => const [];

  void add(String id) => state = [id, ...state.where((e) => e != id)];
}

final shadowIdsProvider =
    NotifierProvider<ShadowIdsNotifier, List<String>>(ShadowIdsNotifier.new);
