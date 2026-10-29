import 'models.dart';

/// Everything the app needs from the backend. Implemented by the HTTP client,
/// the demo fixtures, and the fallback wrapper.
abstract class TraceApi {
  Future<AuthResult> googleSignIn(String idToken);
  Future<void> logout();
  Future<void> deleteAccount();
  Future<Me> me();

  Future<List<WatchlistItem>> watchlists();
  Future<void> addWatch(Chain chain, String address, {String? alias});
  Future<void> removeWatch(String id);
  Future<AlertsPage> alerts({String? cursor});

  Future<MoversResponse> movers({String tab, String window, Chain? chain});
  Future<WalletsResponse> discoveryWallets({String tab, Chain? chain, String? cursor});

  Future<WalletDetail> wallet(Chain chain, String address);
  Future<TradesResponse> walletTrades(Chain chain, String address, {String? cursor});
  Future<WalletMetricsResponse> walletMetrics(Chain chain, String address);
  Future<DnaResponse> walletDna(Chain chain, String address);

  Future<TokenDetail> token(Chain chain, String address);
  Future<TimelineResponse> tokenTimeline(Chain chain, String address);
  Future<CompareResponse> compare(TokenRef a, TokenRef b);

  Future<String> createShadow(ShadowRequest request);
  Future<ShadowResults> shadowResults(String id);

  Future<BriefingResponse> briefings();
}
