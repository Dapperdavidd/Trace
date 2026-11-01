import '../core/format.dart';
import '../core/json_utils.dart';

// ---------------------------------------------------------------------------
// Enums. Every parser is tolerant: unknown values map to a safe fallback.
// ---------------------------------------------------------------------------

enum Chain {
  solana('solana', 'Solana'),
  ethereum('ethereum', 'Ethereum'),
  base('base', 'Base'),
  bnb('bnb', 'BNB Chain'),
  unknown('unknown', 'Unknown chain');

  const Chain(this.wire, this.label);
  final String wire;
  final String label;

  bool get isEvm => this == ethereum || this == base || this == bnb;

  static const List<Chain> supported = [solana, ethereum, base, bnb];

  static Chain parse(Object? v) {
    final s = strOrNull(v)?.toLowerCase();
    for (final c in values) {
      if (c.wire == s) return c;
    }
    return unknown;
  }
}

enum DataStatus {
  live('live'),
  stale('stale'),
  unavailable('unavailable'),
  partial('partial'),
  demo('demo');

  const DataStatus(this.wire);
  final String wire;

  /// Missing or unknown status is treated as unavailable, never as live.
  static DataStatus parse(Object? v) {
    final s = strOrNull(v)?.toLowerCase();
    for (final d in values) {
      if (d.wire == s) return d;
    }
    return unavailable;
  }
}

enum RiskState {
  pass('pass'),
  warn('warn'),
  block('block'),
  unknown('unknown');

  const RiskState(this.wire);
  final String wire;

  static RiskState parse(Object? v) {
    final s = strOrNull(v)?.toLowerCase();
    for (final r in values) {
      if (r.wire == s) return r;
    }
    return unknown;
  }
}

enum WatchStatus {
  active('active'),
  paused('paused'),
  partialMonitoring('partial_monitoring'),
  unsupportedProtocol('unsupported_protocol'),
  waitingForActivity('waiting_for_activity'),
  unknown('unknown');

  const WatchStatus(this.wire);
  final String wire;

  static WatchStatus parse(Object? v) {
    final s = strOrNull(v);
    for (final w in values) {
      if (w.wire == s) return w;
    }
    return unknown;
  }
}

enum AlertType {
  walletBuy('wallet_buy'),
  walletSell('wallet_sell'),
  riskEvent('risk_event'),
  signal('signal'),
  unknown('unknown');

  const AlertType(this.wire);
  final String wire;

  static AlertType parse(Object? v) {
    final s = strOrNull(v);
    for (final a in values) {
      if (a.wire == s) return a;
    }
    return unknown;
  }
}

enum WalletDecision {
  eligible('eligible'),
  watchOnly('watch_only'),
  insufficientEvidence('insufficient_evidence'),
  rejected('rejected'),
  unknown('unknown');

  const WalletDecision(this.wire);
  final String wire;

  static WalletDecision parse(Object? v) {
    final s = strOrNull(v);
    for (final d in values) {
      if (d.wire == s) return d;
    }
    return unknown;
  }
}

enum PnlConfidence {
  high('high'),
  medium('medium'),
  low('low'),
  insufficient('insufficient');

  const PnlConfidence(this.wire);
  final String wire;

  /// Unknown confidence is treated as insufficient, the most cautious value.
  static PnlConfidence parse(Object? v) {
    final s = strOrNull(v)?.toLowerCase();
    for (final p in values) {
      if (p.wire == s) return p;
    }
    return insufficient;
  }
}

enum EvidenceKind {
  fact('fact'),
  hypothesis('hypothesis');

  const EvidenceKind(this.wire);
  final String wire;

  /// Unknown kind is shown as a hypothesis, never as a fact.
  static EvidenceKind parse(Object? v) =>
      strOrNull(v) == 'fact' ? fact : hypothesis;
}

// ---------------------------------------------------------------------------
// Honesty envelope and refs
// ---------------------------------------------------------------------------

class Envelope {
  const Envelope({
    required this.status,
    this.asOf,
    this.demo = false,
    this.coverageNote,
  });

  final DataStatus status;
  final DateTime? asOf;
  final bool demo;
  final String? coverageNote;

  bool get isDemo => demo || status == DataStatus.demo;

  factory Envelope.fromJson(Map<String, dynamic> j) {
    final demo = boolOr(j['demo']);
    var status = DataStatus.parse(j['data_status']);
    if (demo) status = DataStatus.demo;
    return Envelope(
      status: status,
      asOf: dateOrNull(j['as_of']),
      demo: demo,
      coverageNote: strOrNull(j['coverage_note']),
    );
  }
}

class TokenRef {
  const TokenRef({
    required this.chain,
    required this.address,
    this.symbol,
    this.name,
  });

  final Chain chain;
  final String address;
  final String? symbol;
  final String? name;

  factory TokenRef.fromJson(Map<String, dynamic> j) => TokenRef(
        chain: Chain.parse(j['chain']),
        address: str(j['address']),
        symbol: strOrNull(j['symbol']),
        name: strOrNull(j['name']),
      );

  String get displayName => symbol ?? name ?? shortAddress(address);
  String get path => '/token/${chain.wire}/$address';
}

class WalletRef {
  const WalletRef({required this.chain, required this.address, this.alias});

  final Chain chain;
  final String address;
  final String? alias;

  factory WalletRef.fromJson(Map<String, dynamic> j) => WalletRef(
        chain: Chain.parse(j['chain']),
        address: str(j['address']),
        alias: strOrNull(j['alias']),
      );

  String get displayName => alias ?? shortAddress(address);
  String get path => '/wallet-profile/${chain.wire}/$address';
}

// ---------------------------------------------------------------------------
// Auth and account
// ---------------------------------------------------------------------------

class AuthUser {
  const AuthUser({required this.id, this.email, this.name});
  final String id;
  final String? email;
  final String? name;

  factory AuthUser.fromJson(Map<String, dynamic> j) => AuthUser(
        id: str(j['id']),
        email: strOrNull(j['email']),
        name: strOrNull(j['name']),
      );
}

class AuthResult {
  const AuthResult(
      {required this.accessToken, this.expiresAt, required this.user});
  final String accessToken;
  final DateTime? expiresAt;
  final AuthUser user;

  factory AuthResult.fromJson(Map<String, dynamic> j) => AuthResult(
        accessToken: str(j['access_token']),
        expiresAt: dateOrNull(j['expires_at']),
        user: AuthUser.fromJson(asMap(j['user'])),
      );
}

class Me {
  const Me({
    required this.id,
    this.email,
    this.name,
    this.watchOnlyWallets = 0,
    this.connectedWallets = 0,
  });
  final String id;
  final String? email;
  final String? name;
  final int watchOnlyWallets;
  final int connectedWallets;

  factory Me.fromJson(Map<String, dynamic> j) {
    final w = asMap(j['wallets']);
    return Me(
      id: str(j['id']),
      email: strOrNull(j['email']),
      name: strOrNull(j['name']),
      watchOnlyWallets: intOr(w['watch_only']),
      connectedWallets: intOr(w['connected']),
    );
  }
}

// ---------------------------------------------------------------------------
// Watchlist and alerts
// ---------------------------------------------------------------------------

class WatchlistItem {
  const WatchlistItem({
    required this.id,
    required this.wallet,
    required this.status,
    this.lastCheckedAt,
  });
  final String id;
  final WalletRef wallet;
  final WatchStatus status;
  final DateTime? lastCheckedAt;

  factory WatchlistItem.fromJson(Map<String, dynamic> j) => WatchlistItem(
        id: str(j['id']),
        wallet: WalletRef.fromJson(asMap(j['wallet'])),
        status: WatchStatus.parse(j['status']),
        lastCheckedAt: dateOrNull(j['last_checked_at']),
      );
}

class AlertItem {
  const AlertItem({
    required this.id,
    required this.type,
    required this.title,
    required this.body,
    this.evidenceTx = const [],
    this.eventTime,
    this.publishedAt,
  });
  final String id;
  final AlertType type;
  final String title;
  final String body;
  final List<String> evidenceTx;
  final DateTime? eventTime;
  final DateTime? publishedAt;

  factory AlertItem.fromJson(Map<String, dynamic> j) => AlertItem(
        id: str(j['id']),
        type: AlertType.parse(j['type']),
        title: str(j['title']),
        body: str(j['body']),
        evidenceTx: strList(j['evidence_tx']),
        eventTime: dateOrNull(j['event_time']),
        publishedAt: dateOrNull(j['published_at']),
      );
}

class AlertsPage {
  const AlertsPage({required this.items, this.nextCursor});
  final List<AlertItem> items;
  final String? nextCursor;

  factory AlertsPage.fromJson(Map<String, dynamic> j) => AlertsPage(
        items: mapList(j['items'], AlertItem.fromJson),
        nextCursor: strOrNull(j['next_cursor']),
      );
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

class MoverItem {
  const MoverItem({
    required this.rank,
    required this.token,
    this.priceUsd,
    this.changePct,
    this.volumeUsd,
    this.liquidityUsd,
    this.pool = '',
    this.reasonCode = '',
    this.reasonLabel = '',
    this.sparkline = const [],
    this.risk = RiskState.unknown,
    this.riskNotes = const [],
    this.sourceTimestamp,
  });
  final int rank;
  final TokenRef token;
  final String? priceUsd;
  final String? changePct;
  final String? volumeUsd;
  final String? liquidityUsd;
  final String pool;
  final String reasonCode;
  final String reasonLabel;
  final List<double> sparkline;
  final RiskState risk;
  final List<String> riskNotes;
  final DateTime? sourceTimestamp;

  factory MoverItem.fromJson(Map<String, dynamic> j) {
    final reason = asMap(j['reason']);
    return MoverItem(
      rank: intOr(j['rank']),
      token: TokenRef.fromJson(asMap(j['token'])),
      priceUsd: strOrNull(j['price_usd']),
      changePct: strOrNull(j['change_pct']),
      volumeUsd: strOrNull(j['volume_usd']),
      liquidityUsd: strOrNull(j['liquidity_usd']),
      pool: str(j['pool']),
      reasonCode: str(reason['code']),
      reasonLabel: str(reason['label']),
      sparkline: doubleList(j['sparkline']),
      risk: RiskState.parse(j['risk']),
      riskNotes: strList(j['risk_notes']),
      sourceTimestamp: dateOrNull(j['source_timestamp']),
    );
  }
}

class MoversResponse {
  const MoversResponse({required this.env, required this.items});
  final Envelope env;
  final List<MoverItem> items;

  factory MoversResponse.fromJson(Map<String, dynamic> j) => MoversResponse(
        env: Envelope.fromJson(j),
        items: mapList(j['items'], MoverItem.fromJson),
      );
}

class WalletSummary {
  const WalletSummary({
    required this.wallet,
    this.realizedReturnPct,
    this.winRatePct,
    this.sampleSize = 0,
    this.tags = const [],
    this.decision = WalletDecision.unknown,
    this.pnlConfidence = PnlConfidence.insufficient,
    this.coverageRatio,
  });
  final WalletRef wallet;
  final String? realizedReturnPct;
  final String? winRatePct;
  final int sampleSize;
  final List<String> tags;
  final WalletDecision decision;
  final PnlConfidence pnlConfidence;
  final String? coverageRatio;

  factory WalletSummary.fromJson(Map<String, dynamic> j) => WalletSummary(
        wallet: WalletRef.fromJson(asMap(j['wallet'])),
        realizedReturnPct: strOrNull(j['realized_return_pct']),
        winRatePct: strOrNull(j['win_rate_pct']),
        sampleSize: intOr(j['sample_size']),
        tags: strList(j['tags']),
        decision: WalletDecision.parse(j['decision']),
        pnlConfidence: PnlConfidence.parse(j['pnl_confidence']),
        coverageRatio: strOrNull(j['coverage_ratio']),
      );
}

class WalletsResponse {
  const WalletsResponse(
      {required this.env, required this.items, this.nextCursor});
  final Envelope env;
  final List<WalletSummary> items;
  final String? nextCursor;

  factory WalletsResponse.fromJson(Map<String, dynamic> j) => WalletsResponse(
        env: Envelope.fromJson(j),
        items: mapList(j['items'], WalletSummary.fromJson),
        nextCursor: strOrNull(j['next_cursor']),
      );
}

// ---------------------------------------------------------------------------
// Wallet research
// ---------------------------------------------------------------------------

class Metrics {
  const Metrics({
    this.realizedPnlUsd,
    this.unrealizedPnlUsd,
    this.totalTrades = 0,
    this.coverageRatio,
    this.pnlConfidence = PnlConfidence.insufficient,
    this.unresolvedDisposals = 0,
  });
  final String? realizedPnlUsd;
  final String? unrealizedPnlUsd;
  final int totalTrades;
  final String? coverageRatio;
  final PnlConfidence pnlConfidence;
  final int unresolvedDisposals;

  factory Metrics.fromJson(Map<String, dynamic> j) => Metrics(
        realizedPnlUsd: strOrNull(j['realized_pnl_usd']),
        unrealizedPnlUsd: strOrNull(j['unrealized_pnl_usd']),
        totalTrades: intOr(j['total_trades']),
        coverageRatio: strOrNull(j['coverage_ratio']),
        pnlConfidence: PnlConfidence.parse(j['pnl_confidence']),
        unresolvedDisposals: intOr(j['unresolved_disposals']),
      );
}

class Score {
  const Score({
    this.decision = '',
    this.value,
    this.version = '',
    this.components = const {},
    this.cohort = '',
    this.window = '',
    this.explanation = '',
  });
  final String decision;
  final double? value;
  final String version;
  final Map<String, String> components;
  final String cohort;
  final String window;
  final String explanation;

  factory Score.fromJson(Map<String, dynamic> j) => Score(
        decision: str(j['decision']),
        value: doubleOrNull(j['value']),
        version: str(j['version']),
        components: stringMap(j['components']),
        cohort: str(j['cohort']),
        window: str(j['window']),
        explanation: str(j['explanation']),
      );

  static Score? fromAny(Object? v) =>
      v is Map ? Score.fromJson(asMap(v)) : null;
}

class WalletDetail {
  const WalletDetail({
    required this.env,
    required this.wallet,
    this.firstSeen,
    this.lastSeen,
    this.monitoring = '',
    required this.metrics,
  });
  final Envelope env;
  final WalletRef wallet;
  final DateTime? firstSeen;
  final DateTime? lastSeen;
  final String monitoring;
  final Metrics metrics;

  factory WalletDetail.fromJson(Map<String, dynamic> j) => WalletDetail(
        env: Envelope.fromJson(j),
        wallet: WalletRef.fromJson(asMap(j['wallet'])),
        firstSeen: dateOrNull(j['first_seen']),
        lastSeen: dateOrNull(j['last_seen']),
        monitoring: str(j['monitoring']),
        metrics: Metrics.fromJson(asMap(j['metrics'])),
      );
}

enum Finality { tentative, final_ }

class TradeItem {
  const TradeItem({
    required this.txId,
    this.eventTime,
    required this.isBuy,
    required this.token,
    this.amountUsd,
    this.finality = Finality.tentative,
  });
  final String txId;
  final DateTime? eventTime;
  final bool isBuy;
  final TokenRef token;
  final String? amountUsd;
  final Finality finality;

  factory TradeItem.fromJson(Map<String, dynamic> j) => TradeItem(
        txId: str(j['tx_id']),
        eventTime: dateOrNull(j['event_time']),
        isBuy: str(j['side']).toLowerCase() != 'sell',
        token: TokenRef.fromJson(asMap(j['token'])),
        amountUsd: strOrNull(j['amount_usd']),
        // Unknown finality is treated as tentative.
        finality: str(j['finality']) == 'final'
            ? Finality.final_
            : Finality.tentative,
      );
}

class TradesResponse {
  const TradesResponse(
      {required this.env, required this.items, this.nextCursor});
  final Envelope env;
  final List<TradeItem> items;
  final String? nextCursor;

  factory TradesResponse.fromJson(Map<String, dynamic> j) => TradesResponse(
        env: Envelope.fromJson(j),
        items: mapList(j['items'], TradeItem.fromJson),
        nextCursor: strOrNull(j['next_cursor']),
      );
}

class WalletMetricsResponse {
  const WalletMetricsResponse({
    required this.env,
    required this.metrics,
    this.wqs,
    this.wcs,
  });
  final Envelope env;
  final Metrics metrics;
  final Score? wqs;
  final Score? wcs;

  factory WalletMetricsResponse.fromJson(Map<String, dynamic> j) =>
      WalletMetricsResponse(
        env: Envelope.fromJson(j),
        metrics: Metrics.fromJson(j),
        wqs: Score.fromAny(j['wqs']),
        wcs: Score.fromAny(j['wcs']),
      );
}

class DnaTrait {
  const DnaTrait(
      {required this.key, required this.label, required this.evidence});
  final String key;
  final String label;
  final String evidence;

  factory DnaTrait.fromJson(Map<String, dynamic> j) => DnaTrait(
        key: str(j['key']),
        label: str(j['label']),
        evidence: str(j['evidence']),
      );
}

class DnaResponse {
  const DnaResponse({
    required this.env,
    required this.insufficientHistory,
    this.rulesVersion = '',
    this.sampleSize = 0,
    this.traits = const [],
  });
  final Envelope env;
  final bool insufficientHistory;
  final String rulesVersion;
  final int sampleSize;
  final List<DnaTrait> traits;

  factory DnaResponse.fromJson(Map<String, dynamic> j) {
    final state = str(j['state']);
    final traits = mapList(j['traits'], DnaTrait.fromJson);
    // Anything other than an explicit "ok" is treated as insufficient history.
    return DnaResponse(
      env: Envelope.fromJson(j),
      insufficientHistory: state != 'ok',
      rulesVersion: str(j['rules_version']),
      sampleSize: intOr(j['sample_size']),
      traits: state == 'ok' ? traits : const [],
    );
  }
}

// ---------------------------------------------------------------------------
// Tokens
// ---------------------------------------------------------------------------

class RiskCheck {
  const RiskCheck({required this.key, required this.state, this.note = ''});
  final String key;
  final RiskState state;
  final String note;

  factory RiskCheck.fromJson(Map<String, dynamic> j) => RiskCheck(
        key: str(j['key']),
        state: RiskState.parse(j['state']),
        note: str(j['note']),
      );

  String get label {
    switch (key) {
      case 'liquidity':
        return 'Liquidity';
      case 'honeypot':
        return 'Honeypot risk';
      case 'holder_concentration':
        return 'Large holder concentration';
      case 'contract_safety':
        return 'Contract safety';
      case 'suspicious_activity':
        return 'Suspicious activity';
      default:
        return key.isEmpty ? 'Check' : key.replaceAll('_', ' ');
    }
  }
}

class TokenRisk {
  const TokenRisk({
    this.overall = RiskState.unknown,
    this.checks = const [],
    this.fetchedAt,
  });
  final RiskState overall;
  final List<RiskCheck> checks;
  final DateTime? fetchedAt;

  factory TokenRisk.fromJson(Map<String, dynamic> j) => TokenRisk(
        overall: RiskState.parse(j['overall']),
        checks: mapList(j['checks'], RiskCheck.fromJson),
        fetchedAt: dateOrNull(j['fetched_at']),
      );
}

class WhyMoving {
  const WhyMoving(
      {required this.kind, required this.text, this.evidenceIds = const []});
  final EvidenceKind kind;
  final String text;
  final List<String> evidenceIds;

  factory WhyMoving.fromJson(Map<String, dynamic> j) => WhyMoving(
        kind: EvidenceKind.parse(j['kind']),
        text: str(j['text']),
        evidenceIds: strList(j['evidence_ids']),
      );
}

class TokenDetail {
  const TokenDetail({
    required this.env,
    required this.token,
    this.priceUsd,
    this.changePct24h,
    this.marketCapUsd,
    this.volume24hUsd,
    this.liquidityUsd,
    this.holders,
    required this.risk,
    this.whyMoving = const [],
    this.buyers = const [],
  });
  final Envelope env;
  final TokenRef token;
  final String? priceUsd;
  final String? changePct24h;
  final String? marketCapUsd;
  final String? volume24hUsd;
  final String? liquidityUsd;
  final int? holders;
  final TokenRisk risk;
  final List<WhyMoving> whyMoving;
  final List<WalletRef> buyers;

  factory TokenDetail.fromJson(Map<String, dynamic> j) => TokenDetail(
        env: Envelope.fromJson(j),
        token: TokenRef.fromJson(asMap(j['token'])),
        priceUsd: strOrNull(j['price_usd']),
        changePct24h: strOrNull(j['change_pct_24h']),
        marketCapUsd: strOrNull(j['market_cap_usd']),
        volume24hUsd: strOrNull(j['volume_24h_usd']),
        liquidityUsd: strOrNull(j['liquidity_usd']),
        holders: intOrNull(j['holders']),
        risk: TokenRisk.fromJson(asMap(j['risk'])),
        whyMoving: mapList(j['why_moving'], WhyMoving.fromJson),
        buyers: mapList(j['buyers'], WalletRef.fromJson),
      );
}

class TimelineItem {
  const TimelineItem({
    this.time,
    required this.kind,
    required this.title,
    this.detail = '',
    this.evidenceIds = const [],
    this.possibleImpact,
  });
  final DateTime? time;
  final EvidenceKind kind;
  final String title;
  final String detail;
  final List<String> evidenceIds;
  final String? possibleImpact;

  factory TimelineItem.fromJson(Map<String, dynamic> j) => TimelineItem(
        time: dateOrNull(j['time']),
        kind: EvidenceKind.parse(j['kind']),
        title: str(j['title']),
        detail: str(j['detail']),
        evidenceIds: strList(j['evidence_ids']),
        possibleImpact: strOrNull(j['possible_impact']),
      );
}

class TimelineResponse {
  const TimelineResponse({required this.env, required this.items});
  final Envelope env;
  final List<TimelineItem> items;

  factory TimelineResponse.fromJson(Map<String, dynamic> j) => TimelineResponse(
        env: Envelope.fromJson(j),
        items: mapList(j['items'], TimelineItem.fromJson),
      );
}

/// The contract leaves TokenSummary undefined. This model assumes it mirrors the
/// token detail headline fields and parses tolerantly (see README ambiguities).
class TokenSummary {
  const TokenSummary({
    required this.token,
    this.priceUsd,
    this.changePct24h,
    this.marketCapUsd,
    this.volume24hUsd,
    this.liquidityUsd,
    this.risk = RiskState.unknown,
  });
  final TokenRef token;
  final String? priceUsd;
  final String? changePct24h;
  final String? marketCapUsd;
  final String? volume24hUsd;
  final String? liquidityUsd;
  final RiskState risk;

  factory TokenSummary.fromJson(Map<String, dynamic> j) {
    final riskRaw = j['risk'];
    final risk = riskRaw is Map
        ? RiskState.parse(asMap(riskRaw)['overall'])
        : RiskState.parse(riskRaw);
    return TokenSummary(
      token: TokenRef.fromJson(asMap(j['token'])),
      priceUsd: strOrNull(j['price_usd']),
      changePct24h: strOrNull(j['change_pct_24h'] ?? j['change_pct']),
      marketCapUsd: strOrNull(j['market_cap_usd']),
      volume24hUsd: strOrNull(j['volume_24h_usd'] ?? j['volume_usd']),
      liquidityUsd: strOrNull(j['liquidity_usd']),
      risk: risk,
    );
  }
}

class CompareResponse {
  const CompareResponse(
      {required this.env, required this.items, this.verdictNote = ''});
  final Envelope env;
  final List<TokenSummary> items;
  final String verdictNote;

  factory CompareResponse.fromJson(Map<String, dynamic> j) => CompareResponse(
        env: Envelope.fromJson(j),
        items: mapList(j['items'], TokenSummary.fromJson),
        verdictNote: str(j['verdict_note']),
      );
}

// ---------------------------------------------------------------------------
// Shadow strategies (paper only)
// ---------------------------------------------------------------------------

class ShadowRequest {
  const ShadowRequest({
    required this.chain,
    required this.address,
    required this.budgetUsd,
    required this.sizeRule,
    required this.delaySeconds,
    required this.slippageBps,
    required this.exitRule,
  });
  final Chain chain;
  final String address;
  final String budgetUsd;
  final String sizeRule;
  final int delaySeconds;
  final int slippageBps;
  final String exitRule;

  Map<String, dynamic> toJson() => {
        'wallet': {'chain': chain.wire, 'address': address},
        'budget_usd': budgetUsd,
        'size_rule': sizeRule,
        'delay_seconds': delaySeconds,
        'slippage_bps': slippageBps,
        'exit_rule': exitRule,
      };
}

class ShadowResults {
  const ShadowResults({
    required this.env,
    this.startingUsd,
    this.endingUsd,
    this.returnPct,
    this.trades = 0,
    this.winRatePct,
    this.skippedUnexecutable = 0,
    this.feesAndSlippageUsd,
    this.baseline = const {},
    this.assumptions = const {},
  });
  final Envelope env;
  final String? startingUsd;
  final String? endingUsd;
  final String? returnPct;
  final int trades;
  final String? winRatePct;
  final int skippedUnexecutable;
  final String? feesAndSlippageUsd;
  final Map<String, String> baseline;
  final Map<String, String> assumptions;

  factory ShadowResults.fromJson(Map<String, dynamic> j) => ShadowResults(
        env: Envelope.fromJson(j),
        startingUsd: strOrNull(j['starting_usd']),
        endingUsd: strOrNull(j['ending_usd']),
        returnPct: strOrNull(j['return_pct']),
        trades: intOr(j['trades']),
        winRatePct: strOrNull(j['win_rate_pct']),
        skippedUnexecutable: intOr(j['skipped_unexecutable']),
        feesAndSlippageUsd: strOrNull(j['fees_and_slippage_usd']),
        baseline: stringMap(j['baseline']),
        assumptions: stringMap(j['assumptions']),
      );
}

// ---------------------------------------------------------------------------
// Briefings
// ---------------------------------------------------------------------------

class BriefingItem {
  const BriefingItem({required this.text, this.link});
  final String text;
  final String? link;

  factory BriefingItem.fromJson(Map<String, dynamic> j) =>
      BriefingItem(text: str(j['text']), link: strOrNull(j['link']));
}

class BriefingSection {
  const BriefingSection({required this.title, required this.items});
  final String title;
  final List<BriefingItem> items;

  factory BriefingSection.fromJson(Map<String, dynamic> j) => BriefingSection(
        title: str(j['title']),
        items: mapList(j['items'], BriefingItem.fromJson),
      );
}

class BriefingResponse {
  const BriefingResponse({required this.env, required this.sections});
  final Envelope env;
  final List<BriefingSection> sections;

  factory BriefingResponse.fromJson(Map<String, dynamic> j) => BriefingResponse(
        env: Envelope.fromJson(j),
        sections: mapList(j['sections'], BriefingSection.fromJson),
      );
}

class ProtocolCoverage {
  const ProtocolCoverage(
      {required this.protocol, required this.status, this.blocker});
  final String protocol;
  final String status;
  final String? blocker;

  factory ProtocolCoverage.fromJson(Map<String, dynamic> j) => ProtocolCoverage(
        protocol: str(j['protocol']),
        status: strOrNull(j['status']) ?? 'unsupported',
        blocker: strOrNull(j['blocker']),
      );
}

class ChainCoverage {
  const ChainCoverage(
      {required this.chain,
      required this.name,
      required this.status,
      required this.protocols});
  final Chain chain;
  final String name;
  final String status;
  final List<ProtocolCoverage> protocols;

  factory ChainCoverage.fromJson(Map<String, dynamic> j) => ChainCoverage(
        chain: Chain.parse(j['chain']),
        name: strOrNull(j['display_name']) ?? Chain.parse(j['chain']).label,
        status: strOrNull(j['status']) ?? 'unsupported',
        protocols: mapList(j['protocols'], ProtocolCoverage.fromJson),
      );
}

class CoverageResponse {
  const CoverageResponse(
      {required this.env, required this.chains, required this.note});
  final Envelope env;
  final List<ChainCoverage> chains;
  final String note;

  factory CoverageResponse.fromJson(Map<String, dynamic> j) => CoverageResponse(
        env: Envelope.fromJson(j),
        chains: mapList(j['chains'], ChainCoverage.fromJson),
        note: str(j['note']),
      );
}
