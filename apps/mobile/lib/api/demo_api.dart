import 'api_error.dart';
import 'models.dart';
import 'trace_api.dart';

/// Fictional demo fixtures. Every analytic response is flagged demo and is
/// built through the same parsers as live responses. Token names, addresses
/// and numbers are invented for illustration and are not market data.
class DemoApi implements TraceApi {
  DemoApi({DateTime Function()? clock}) : _now = clock ?? DateTime.now;

  final DateTime Function() _now;

  /// In-memory demo watchlist. Lost when the app restarts.
  final List<WatchlistItem> _watch = [];
  bool _seeded = false;
  int _nextId = 1;
  int _nextShadow = 1;

  static const String demoWalletAddress =
      '7xAq3Fz9wLm2Rt8YbKd4VnHc6PjUe5SgXa1NoWi9kP';
  static const String demoThinWalletAddress =
      '9QpLr4TnVs2Wd7XcHg5MbKj8EyAu3FzNoRi6Dk1ePt';

  static String _fakeSol(int seed) {
    const a = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
    final b = StringBuffer();
    var x = seed * 7919 + 13;
    for (var i = 0; i < 44; i++) {
      x = (x * 1103515245 + 12345) & 0x7fffffff;
      b.write(a[x % a.length]);
    }
    return b.toString();
  }

  static final Map<String, String> _addr = {
    'NEURA': _fakeSol(1),
    'SOLX': _fakeSol(2),
    'FROG': _fakeSol(3),
    'NOVA': _fakeSol(4),
    'ORBIT': _fakeSol(5),
    'ZETA': _fakeSol(6),
    'PIXEL': _fakeSol(7),
    'MIND': _fakeSol(8),
  };

  static const Map<String, String> _names = {
    'NEURA': 'Neura (demo)',
    'SOLX': 'Solx (demo)',
    'FROG': 'Frog (demo)',
    'NOVA': 'Nova (demo)',
    'ORBIT': 'Orbit (demo)',
    'ZETA': 'Zeta (demo)',
    'PIXEL': 'Pixel (demo)',
    'MIND': 'Mind (demo)',
  };

  /// Address of a demo token by symbol, for tests and navigation defaults.
  static String addressOf(String symbol) => _addr[symbol] ?? _fakeSol(99);

  Map<String, dynamic> _env({String? note}) => {
        'data_status': 'demo',
        'as_of': _now().toUtc().toIso8601String(),
        'demo': true,
        'coverage_note': note ??
            'Demo data. Fictional tokens and wallets. Not live market data.',
      };

  Map<String, dynamic> _tok(String sym) => {
        'chain': 'solana',
        'address': addressOf(sym),
        'symbol': sym,
        'name': _names[sym],
      };

  String _ago(Duration d) => _now().toUtc().subtract(d).toIso8601String();

  // Auth ---------------------------------------------------------------------

  @override
  Future<AuthResult> googleSignIn(String idToken) async =>
      throw const ApiException(
        kind: ApiFailureKind.http,
        code: 'demo_mode',
        message:
            'Sign-in is not available in demo mode. Use Explore without signing in.',
      );

  @override
  Future<void> logout() async {}

  @override
  Future<void> deleteAccount() async {}

  @override
  Future<Me> me() async => Me.fromJson({
        'id': 'demo-user',
        'email': 'demo@example.invalid',
        'name': 'Demo explorer',
        'wallets': {'watch_only': _watch.length, 'connected': 0},
      });

  // Watchlist and alerts -----------------------------------------------------

  void _seed() {
    if (_seeded) return;
    _seeded = true;
    _watch.add(WatchlistItem.fromJson({
      'id': 'demo-w${_nextId++}',
      'wallet': {
        'chain': 'solana',
        'address': demoWalletAddress,
        'alias': 'Demo trader'
      },
      'status': 'active',
      'last_checked_at': _ago(const Duration(minutes: 2)),
    }));
    _watch.add(WatchlistItem.fromJson({
      'id': 'demo-w${_nextId++}',
      'wallet': {
        'chain': 'solana',
        'address': demoThinWalletAddress,
        'alias': null
      },
      'status': 'waiting_for_activity',
      'last_checked_at': _ago(const Duration(minutes: 9)),
    }));
  }

  @override
  Future<List<WatchlistItem>> watchlists() async {
    _seed();
    return List.unmodifiable(_watch);
  }

  @override
  Future<void> addWatch(Chain chain, String address, {String? alias}) async {
    _seed();
    final exists = _watch
        .any((w) => w.wallet.chain == chain && w.wallet.address == address);
    if (exists) return; // idempotent on chain and address
    _watch.add(WatchlistItem.fromJson({
      'id': 'demo-w${_nextId++}',
      'wallet': {'chain': chain.wire, 'address': address, 'alias': alias},
      'status': 'waiting_for_activity',
      'last_checked_at': _ago(Duration.zero),
    }));
  }

  @override
  Future<void> removeWatch(String id) async {
    _seed();
    _watch.removeWhere((w) => w.id == id);
  }

  @override
  Future<AlertsPage> alerts({String? cursor}) async => AlertsPage.fromJson({
        'items': [
          {
            'id': 'demo-a1',
            'type': 'wallet_buy',
            'title': 'Demo trader bought NEURA',
            'body':
                'A watched wallet bought about \$127K of NEURA. Observed on-chain (demo).',
            'evidence_tx': ['demo-tx-0001'],
            'event_time': _ago(const Duration(minutes: 14)),
            'published_at': _ago(const Duration(minutes: 13)),
          },
          {
            'id': 'demo-a2',
            'type': 'risk_event',
            'title': 'Risk check changed on FROG',
            'body': 'Holder concentration moved from pass to warn (demo).',
            'evidence_tx': <String>[],
            'event_time': _ago(const Duration(hours: 3)),
            'published_at': _ago(const Duration(hours: 3)),
          },
          {
            'id': 'demo-a3',
            'type': 'wallet_sell',
            'title': 'Demo trader sold FROG',
            'body': 'A watched wallet sold about \$45K of FROG (demo).',
            'evidence_tx': ['demo-tx-0002'],
            'event_time': _ago(const Duration(days: 1)),
            'published_at': _ago(const Duration(days: 1)),
          },
        ],
        'next_cursor': null,
      });

  // Discovery ----------------------------------------------------------------

  static const List<List<Object>> _moverRows = [
    // symbol, change, volume, liquidity, reason, risk
    [
      'NEURA',
      '42.7',
      '12400000',
      '3100000',
      'smart_buying|3 tracked wallets bought in the last hour',
      'warn'
    ],
    [
      'SOLX',
      '28.4',
      '8100000',
      '4200000',
      'volume_spike|High volume spike',
      'pass'
    ],
    [
      'FROG',
      '19.1',
      '6300000',
      '2200000',
      'accumulation|Large buyers accumulating',
      'warn'
    ],
    [
      'NOVA',
      '15.8',
      '4900000',
      '2900000',
      'social|Mentions rising on social',
      'pass'
    ],
    ['ORBIT', '12.3', '3100000', '1400000', 'unusual|Unusual activity', 'warn'],
    [
      'ZETA',
      '11.9',
      '2700000',
      '1100000',
      'new_pool|Recently created pool',
      'warn'
    ],
    [
      'PIXEL',
      '9.4',
      '2100000',
      '900000',
      'social|Mentions rising on social',
      'pass'
    ],
    [
      'MIND',
      '8.1',
      '1800000',
      '800000',
      'social|Mentions rising on social',
      'unknown'
    ],
  ];

  @override
  Future<MoversResponse> movers({
    String tab = 'trending',
    String window = '24h',
    Chain? chain,
  }) async {
    final rows = _moverRows.toList();
    if (tab == 'new') {
      rows.removeWhere((r) => r[0] != 'ZETA' && r[0] != 'ORBIT');
    } else if (tab == 'smart_money') {
      rows.removeWhere((r) => r[0] != 'NEURA' && r[0] != 'FROG');
    } else if (tab == 'early_momentum') {
      rows.removeWhere((r) => r[0] == 'SOLX');
    }
    if (chain != null && chain != Chain.solana) rows.clear();
    var rank = 1;
    return MoversResponse.fromJson({
      ..._env(),
      'items': [
        for (final r in rows)
          {
            'rank': rank++,
            'token': _tok(r[0] as String),
            'price_usd':
                r[0] == 'NEURA' ? '0.0084' : '0.0${(r[0] as String).length}21',
            'change_pct': r[1],
            'volume_usd': r[2],
            'liquidity_usd': r[3],
            'pool': 'Demo pool (fictional)',
            'reason': {
              'code': (r[4] as String).split('|')[0],
              'label': (r[4] as String).split('|')[1],
            },
            'sparkline': _spark(rank, double.parse(r[1] as String)),
            'risk': r[5],
            'risk_notes': r[5] == 'warn'
                ? [
                    'Liquidity is low compared with volume, so price can swing hard (demo).'
                  ]
                : <String>[],
            'source_timestamp': _ago(const Duration(minutes: 3)),
          },
      ],
    });
  }

  List<double> _spark(int seed, double change) {
    final out = <double>[];
    var v = 100.0;
    for (var i = 0; i < 16; i++) {
      v += (change / 16) + ((i * seed) % 5 - 2) * 0.6;
      out.add(v);
    }
    return out;
  }

  @override
  Future<WalletsResponse> discoveryWallets({
    String tab = 'top',
    Chain? chain,
    String? cursor,
  }) async =>
      WalletsResponse.fromJson({
        ..._env(),
        'items': [
          {
            'wallet': {
              'chain': 'solana',
              'address': demoWalletAddress,
              'alias': 'Demo trader'
            },
            'realized_return_pct': '1324.0',
            'win_rate_pct': '78',
            'sample_size': 432,
            'tags': ['Early Buyer', 'Holds 1-7 days', 'Large Caps'],
            'decision': 'eligible',
            'pnl_confidence': 'medium',
            'coverage_ratio': '0.82',
          },
          {
            'wallet': {
              'chain': 'solana',
              'address': demoThinWalletAddress,
              'alias': null
            },
            'realized_return_pct': null,
            'win_rate_pct': null,
            'sample_size': 6,
            'tags': ['New wallet'],
            'decision': 'insufficient_evidence',
            'pnl_confidence': 'insufficient',
            'coverage_ratio': null,
          },
        ],
        'next_cursor': null,
      });

  // Wallet research ----------------------------------------------------------

  @override
  Future<WalletDetail> wallet(Chain chain, String address) async {
    final thin = address == demoThinWalletAddress;
    return WalletDetail.fromJson({
      ..._env(),
      'wallet': {
        'chain': chain.wire,
        'address': address,
        'alias': thin ? null : 'Demo trader'
      },
      'first_seen': _ago(Duration(days: thin ? 4 : 410)),
      'last_seen': _ago(const Duration(minutes: 14)),
      'monitoring': thin ? 'partial' : 'active',
      'metrics': _metrics(thin),
    });
  }

  Map<String, dynamic> _metrics(bool thin) => thin
      ? {
          'realized_pnl_usd': null,
          'unrealized_pnl_usd': null,
          'total_trades': 6,
          'coverage_ratio': null,
          'pnl_confidence': 'insufficient',
          'unresolved_disposals': 4,
        }
      : {
          'realized_pnl_usd': '182340.55',
          'unrealized_pnl_usd': '12410.20',
          'total_trades': 432,
          'coverage_ratio': '0.82',
          'pnl_confidence': 'medium',
          'unresolved_disposals': 9,
        };

  @override
  Future<TradesResponse> walletTrades(Chain chain, String address,
          {String? cursor}) async =>
      TradesResponse.fromJson({
        ..._env(),
        'items': [
          _trade('demo-tx-0001', 'buy', 'NEURA', '127000',
              const Duration(minutes: 14), 'tentative'),
          _trade('demo-tx-0003', 'buy', 'SOLX', '89000',
              const Duration(hours: 3), 'final'),
          _trade('demo-tx-0002', 'sell', 'FROG', '45000',
              const Duration(days: 1), 'final'),
          _trade('demo-tx-0004', 'buy', 'NOVA', '73000',
              const Duration(days: 2), 'final'),
        ],
        'next_cursor': null,
      });

  Map<String, dynamic> _trade(String id, String side, String sym, String usd,
          Duration ago, String fin) =>
      {
        'tx_id': id,
        'event_time': _ago(ago),
        'side': side,
        'token': _tok(sym),
        'amount_usd': usd,
        'finality': fin,
      };

  @override
  Future<WalletMetricsResponse> walletMetrics(
      Chain chain, String address) async {
    final thin = address == demoThinWalletAddress;
    return WalletMetricsResponse.fromJson({
      ..._env(),
      ..._metrics(thin),
      'wqs': {
        'decision': thin ? 'insufficient_evidence' : 'eligible',
        'value': thin ? null : 71.0,
        'version': 'wqs-demo-1',
        'components':
            thin ? <String, String>{} : {'consistency': '0.7', 'sample': '0.9'},
        'cohort': 'solana active traders (demo)',
        'window': '90d',
        'explanation': thin
            ? 'Too few completed trades to score this wallet.'
            : 'Estimated quality score from completed trades. It describes the past and is not a prediction.',
      },
      'wcs': thin
          ? null
          : {
              'decision': 'ok',
              'value': 64.0,
              'version': 'wcs-demo-1',
              'components': {'priced_trades': '0.82'},
              'cohort': 'solana active traders (demo)',
              'window': '90d',
              'explanation':
                  'Share of trades where GiFTED! could price both sides.',
            },
    });
  }

  @override
  Future<DnaResponse> walletDna(Chain chain, String address) async {
    if (address == demoThinWalletAddress) {
      return DnaResponse.fromJson({
        ..._env(),
        'state': 'insufficient_history',
        'rules_version': 'dna-demo-1',
        'sample_size': 6,
        'traits': <Object>[],
      });
    }
    return DnaResponse.fromJson({
      ..._env(),
      'state': 'ok',
      'rules_version': 'dna-demo-1',
      'sample_size': 432,
      'traits': [
        {
          'key': 'early_buyer',
          'label': 'Early buyer',
          'evidence': 'Bought within 24h of pool creation in 61% of 432 trades.'
        },
        {
          'key': 'hold_window',
          'label': 'Holds 1 to 7 days',
          'evidence': 'Median hold time 2.8 days across 388 closed positions.'
        },
        {
          'key': 'size_band',
          'label': 'Large positions',
          'evidence': 'Median buy size is in the top size band of the cohort.'
        },
      ],
    });
  }

  // Tokens -------------------------------------------------------------------

  @override
  Future<TokenDetail> token(Chain chain, String address) async {
    final sym = _symbolFor(address);
    final isNeura = sym == 'NEURA';
    return TokenDetail.fromJson({
      ..._env(),
      'token': {
        'chain': chain.wire,
        'address': address,
        'symbol': sym,
        'name': _names[sym]
      },
      'price_usd': isNeura ? '0.0084' : '0.0421',
      'change_pct_24h': isNeura ? '42.7' : '28.4',
      'market_cap_usd': isNeura ? '8400000' : '21000000',
      'volume_24h_usd': isNeura ? '12400000' : '8100000',
      'liquidity_usd': isNeura ? '3100000' : '4200000',
      'holders': isNeura ? 12480 : null,
      'risk': {
        'overall': isNeura ? 'warn' : 'pass',
        'checks': [
          {
            'key': 'liquidity',
            'state': 'pass',
            'note': 'Pool liquidity is \$3.1M, enough for small trades (demo).'
          },
          {
            'key': 'honeypot',
            'state': 'pass',
            'note': 'A test sell succeeded in simulation (demo).'
          },
          {
            'key': 'holder_concentration',
            'state': 'warn',
            'note': 'Top 10 holders own about 38% of supply (demo).'
          },
          {
            'key': 'contract_safety',
            'state': 'pass',
            'note': 'No known risky functions found (demo).'
          },
          {
            'key': 'suspicious_activity',
            'state': 'unknown',
            'note': 'Not enough history to judge (demo).'
          },
        ],
        'fetched_at': _ago(const Duration(minutes: 3)),
      },
      'why_moving': [
        {
          'kind': 'fact',
          'text':
              '3 tracked wallets with eligible history bought in the last hour.',
          'evidence_ids': ['demo-tx-0001', 'demo-tx-0005']
        },
        {
          'kind': 'fact',
          'text':
              'Trading volume is about 4x its usual level over the last 24 hours.',
          'evidence_ids': ['demo-pool-snapshot-1']
        },
        {
          'kind': 'hypothesis',
          'text':
              'Rising social mentions may be adding to demand. GiFTED! cannot confirm this is the cause.',
          'evidence_ids': <String>[]
        },
      ],
      'buyers': [
        {
          'chain': 'solana',
          'address': demoWalletAddress,
          'alias': 'Demo trader'
        },
        {'chain': 'solana', 'address': demoThinWalletAddress, 'alias': null},
      ],
    });
  }

  String _symbolFor(String address) {
    for (final e in _addr.entries) {
      if (e.value == address) return e.key;
    }
    return 'NEURA';
  }

  @override
  Future<TimelineResponse> tokenTimeline(Chain chain, String address) async =>
      TimelineResponse.fromJson({
        ..._env(),
        'items': [
          {
            'time': _ago(const Duration(hours: 3)),
            'kind': 'fact',
            'title': 'Unusual activity',
            'detail': 'Volume rose above its normal level.',
            'evidence_ids': ['demo-pool-snapshot-1'],
            'possible_impact': 'More buyers noticing the token.'
          },
          {
            'time': _ago(const Duration(hours: 2, minutes: 45)),
            'kind': 'fact',
            'title': 'Tracked wallets buying',
            'detail': '3 tracked wallets bought.',
            'evidence_ids': ['demo-tx-0001', 'demo-tx-0005'],
            'possible_impact': 'May signal interest from experienced traders.'
          },
          {
            'time': _ago(const Duration(hours: 2, minutes: 20)),
            'kind': 'hypothesis',
            'title': 'Social attention may have increased',
            'detail': 'Mentions started trending. Attribution is uncertain.',
            'evidence_ids': <String>[],
            'possible_impact': 'Could draw short-term buyers. Not confirmed.'
          },
          {
            'time': _ago(const Duration(hours: 2)),
            'kind': 'fact',
            'title': 'Price started moving',
            'detail': 'Up 18.4% in 45 minutes.',
            'evidence_ids': ['demo-pool-snapshot-2'],
            'possible_impact': null
          },
          {
            'time': _ago(const Duration(minutes: 5)),
            'kind': 'fact',
            'title': 'Now',
            'detail': 'Up 42.7% over 24h. Price still moving.',
            'evidence_ids': ['demo-pool-snapshot-3'],
            'possible_impact': null
          },
        ],
      });

  @override
  Future<CompareResponse> compare(TokenRef a, TokenRef b) async {
    Map<String, dynamic> row(TokenRef t) {
      final sym = _symbolFor(t.address);
      final neura = sym == 'NEURA';
      return {
        'token': {
          'chain': t.chain.wire,
          'address': t.address,
          'symbol': sym,
          'name': _names[sym]
        },
        'price_usd': neura ? '0.0084' : '0.0421',
        'change_pct_24h': neura ? '42.7' : '28.4',
        'market_cap_usd': neura ? '8400000' : '21000000',
        'volume_24h_usd': neura ? '12400000' : '8100000',
        'liquidity_usd': neura ? '3100000' : '4200000',
        'risk': neura ? 'warn' : 'pass',
      };
    }

    return CompareResponse.fromJson({
      ..._env(),
      'items': [row(a), row(b)],
      'verdict_note':
          'The first token shows stronger recent momentum and more tracked-wallet buying, but also lower liquidity. This compares observed data. It does not predict future prices.',
    });
  }

  // Shadow -------------------------------------------------------------------

  @override
  Future<String> createShadow(ShadowRequest request) async =>
      'demo-shadow-${_nextShadow++}';

  @override
  Future<ShadowResults> shadowResults(String id) async =>
      ShadowResults.fromJson({
        ..._env(),
        'starting_usd': '100.00',
        'ending_usd': '148.60',
        'return_pct': '48.6',
        'trades': 12,
        'win_rate_pct': '66.7',
        'skipped_unexecutable': 2,
        'fees_and_slippage_usd': '3.20',
        'baseline': {
          'label': 'Buy and hold the same tokens',
          'return_pct': '31.0'
        },
        'assumptions': {
          'delay_seconds': '30',
          'slippage_bps': '100',
          'size_rule': 'fixed_usd',
          'exit_rule': 'mirror_sells',
          'window': 'last 30 days',
        },
      });

  // Briefings ----------------------------------------------------------------

  @override
  Future<BriefingResponse> briefings() async => BriefingResponse.fromJson({
        ..._env(),
        'sections': [
          {
            'title': 'New opportunities',
            'items': [
              {
                'text': '3 tokens show unusual activity (observed, demo).',
                'link': '/market'
              },
            ],
          },
          {
            'title': 'Wallet moves to watch',
            'items': [
              {
                'text':
                    '2 watched wallets traded since yesterday (observed, demo).',
                'link': '/watchlist'
              },
            ],
          },
          {
            'title': 'Risk warnings',
            'items': [
              {
                'text':
                    '1 token you looked at has a new warning. Warnings are not predictions.',
                'link': null
              },
            ],
          },
          {
            'title': 'Yesterday\'s paper results',
            'items': [
              {
                'text':
                    'Your paper simulation moved +8.4%. Simulated, not real money.',
                'link': '/shadow'
              },
            ],
          },
        ],
      });

  @override
  Future<CoverageResponse> coverage() async => CoverageResponse.fromJson({
        'data_status': 'demo',
        'as_of': _now().toUtc().toIso8601String(),
        'demo': true,
        'coverage_note':
            'Coverage statuses are real capability labels; surrounding market data is demo.',
        'note': 'Only Ethereum Uniswap V2 is enabled.',
        'chains': [
          {
            'chain': 'ethereum',
            'display_name': 'Ethereum',
            'status': 'partial',
            'protocols': [
              {'protocol': 'uniswap_v2', 'status': 'enabled', 'blocker': null},
              {
                'protocol': 'uniswap_v3',
                'status': 'unsupported',
                'blocker': 'No adapter is enabled.'
              },
            ],
          },
          for (final chain in ['solana', 'base', 'bnb'])
            {
              'chain': chain,
              'display_name': Chain.parse(chain).label,
              'status': 'disabled',
              'protocols': <Object>[],
            },
        ],
      });
}
