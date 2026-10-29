import 'package:flutter_test/flutter_test.dart';
import 'package:trace_mobile/api/models.dart';
import 'package:trace_mobile/core/json_utils.dart';

void main() {
  group('Envelope', () {
    test('missing data_status is unavailable, never live', () {
      final e = Envelope.fromJson(const {});
      expect(e.status, DataStatus.unavailable);
      expect(e.isDemo, isFalse);
      expect(e.asOf, isNull);
    });
    test('unknown data_status is unavailable', () {
      expect(Envelope.fromJson(const {'data_status': 'weird'}).status, DataStatus.unavailable);
    });
    test('demo flag wins over a live status', () {
      final e = Envelope.fromJson(const {'data_status': 'live', 'demo': true});
      expect(e.status, DataStatus.demo);
      expect(e.isDemo, isTrue);
    });
    test('parses as_of and coverage note', () {
      final e = Envelope.fromJson(const {
        'data_status': 'partial',
        'as_of': '2026-10-28T12:00:00Z',
        'coverage_note': 'Solana only',
      });
      expect(e.status, DataStatus.partial);
      expect(e.asOf, DateTime.utc(2026, 10, 28, 12));
      expect(e.coverageNote, 'Solana only');
    });
  });

  group('tolerant readers', () {
    test('never throw on wrong types', () {
      expect(asMap('x'), isEmpty);
      expect(asList(5), isEmpty);
      expect(intOr('7'), 7);
      expect(intOr('nope', 3), 3);
      expect(doubleOrNull('1.5'), 1.5);
      expect(doubleOrNull(<int>[]), isNull);
      expect(strOrNull(12), '12');
      expect(strOrNull(<String>[]), isNull);
      expect(doubleList(['1', 2, null, 'x']), [1.0, 2.0]);
      expect(strList(['a', 1, null]), ['a', '1']);
    });
  });

  group('MoversResponse', () {
    test('parses full item', () {
      final r = MoversResponse.fromJson({
        'data_status': 'live',
        'as_of': '2026-10-28T12:00:00Z',
        'demo': false,
        'items': [
          {
            'rank': 1,
            'token': {'chain': 'solana', 'address': 'AAA', 'symbol': 'NEURA', 'name': null},
            'price_usd': '0.0084',
            'change_pct': '42.7',
            'volume_usd': '12400000',
            'liquidity_usd': '3100000',
            'pool': 'pool-1',
            'reason': {'code': 'volume_spike', 'label': 'Volume spike'},
            'sparkline': [1, 2, 3.5],
            'risk': 'warn',
            'risk_notes': ['thin liquidity'],
            'source_timestamp': '2026-10-28T11:57:00Z',
          }
        ],
      });
      expect(r.env.status, DataStatus.live);
      final m = r.items.single;
      expect(m.token.chain, Chain.solana);
      expect(m.token.displayName, 'NEURA');
      expect(m.reasonLabel, 'Volume spike');
      expect(m.sparkline, [1.0, 2.0, 3.5]);
      expect(m.risk, RiskState.warn);
      expect(m.token.path, '/token/solana/AAA');
    });
    test('tolerates missing and unknown fields', () {
      final r = MoversResponse.fromJson({
        'data_status': 'live',
        'extra_unknown_field': 1,
        'items': [
          {'rank': '3', 'risk': 'sketchy', 'token': {'chain': 'dogechain', 'address': 'X'}},
          'not a map',
          null,
        ],
      });
      expect(r.items, hasLength(1));
      final m = r.items.single;
      expect(m.rank, 3);
      expect(m.risk, RiskState.unknown);
      expect(m.token.chain, Chain.unknown);
      expect(m.priceUsd, isNull);
      expect(m.sparkline, isEmpty);
      expect(m.token.displayName, 'X');
    });
    test('garbage body becomes an empty unavailable response', () {
      final r = MoversResponse.fromJson(asMap('garbage'));
      expect(r.items, isEmpty);
      expect(r.env.status, DataStatus.unavailable);
    });
  });

  group('Wallet models', () {
    test('summary with unknown enums uses cautious defaults', () {
      final w = WalletSummary.fromJson({
        'wallet': {'chain': 'ethereum', 'address': '0xabc', 'alias': null},
        'pnl_confidence': 'super',
        'decision': 'maybe',
      });
      expect(w.pnlConfidence, PnlConfidence.insufficient);
      expect(w.decision, WalletDecision.unknown);
      expect(w.realizedReturnPct, isNull);
      expect(w.tags, isEmpty);
    });
    test('metrics response merges flat metrics with scores', () {
      final r = WalletMetricsResponse.fromJson({
        'data_status': 'live',
        'realized_pnl_usd': '10.5',
        'unrealized_pnl_usd': null,
        'total_trades': 12,
        'coverage_ratio': '0.5',
        'pnl_confidence': 'low',
        'unresolved_disposals': 2,
        'wqs': {'decision': 'eligible', 'value': 71, 'version': 'v1', 'components': {'a': '0.5'}, 'cohort': 'c', 'window': '90d', 'explanation': 'e'},
        'wcs': null,
      });
      expect(r.metrics.realizedPnlUsd, '10.5');
      expect(r.metrics.unrealizedPnlUsd, isNull);
      expect(r.metrics.pnlConfidence, PnlConfidence.low);
      expect(r.wqs!.value, 71.0);
      expect(r.wqs!.components['a'], '0.5');
      expect(r.wcs, isNull);
    });
    test('DNA insufficient_history has no traits even if sent', () {
      final r = DnaResponse.fromJson({
        'data_status': 'live',
        'state': 'insufficient_history',
        'sample_size': 4,
        'traits': [
          {'key': 'k', 'label': 'L', 'evidence': 'e'}
        ],
      });
      expect(r.insufficientHistory, isTrue);
      expect(r.traits, isEmpty);
      expect(r.sampleSize, 4);
    });
    test('DNA with unknown or missing state is treated as insufficient', () {
      expect(DnaResponse.fromJson(const {}).insufficientHistory, isTrue);
      expect(DnaResponse.fromJson(const {'state': 'new_state'}).insufficientHistory, isTrue);
    });
    test('DNA ok keeps traits', () {
      final r = DnaResponse.fromJson({
        'state': 'ok',
        'rules_version': 'r1',
        'sample_size': 100,
        'traits': [
          {'key': 'k', 'label': 'Early buyer', 'evidence': 'e'}
        ],
      });
      expect(r.insufficientHistory, isFalse);
      expect(r.traits.single.label, 'Early buyer');
    });
    test('trade finality defaults to tentative', () {
      final t = TradeItem.fromJson({'tx_id': 't', 'side': 'sell', 'finality': 'weird'});
      expect(t.isBuy, isFalse);
      expect(t.finality, Finality.tentative);
      expect(TradeItem.fromJson(const {'finality': 'final'}).finality, Finality.final_);
    });
  });

  group('Token models', () {
    test('unknown evidence kind is a hypothesis, never a fact', () {
      final t = TokenDetail.fromJson({
        'data_status': 'live',
        'token': {'chain': 'base', 'address': '0x1', 'symbol': 'X'},
        'why_moving': [
          {'kind': 'fact', 'text': 'a', 'evidence_ids': ['e1']},
          {'kind': 'rumor', 'text': 'b'},
          {'text': 'c'},
        ],
      });
      expect(t.whyMoving[0].kind, EvidenceKind.fact);
      expect(t.whyMoving[1].kind, EvidenceKind.hypothesis);
      expect(t.whyMoving[2].kind, EvidenceKind.hypothesis);
      expect(t.holders, isNull);
      expect(t.risk.overall, RiskState.unknown);
      expect(t.risk.checks, isEmpty);
    });
    test('risk check labels', () {
      expect(RiskCheck.fromJson(const {'key': 'honeypot'}).label, 'Honeypot risk');
      expect(RiskCheck.fromJson(const {'key': 'new_check'}).label, 'new check');
      expect(RiskCheck.fromJson(const {}).label, 'Check');
    });
    test('compare tolerates risk as string or object', () {
      final r = CompareResponse.fromJson({
        'data_status': 'live',
        'items': [
          {'token': {'chain': 'solana', 'address': 'A', 'symbol': 'A'}, 'risk': 'pass', 'change_pct': '1'},
          {'token': {'chain': 'solana', 'address': 'B', 'symbol': 'B'}, 'risk': {'overall': 'warn'}},
        ],
        'verdict_note': 'n',
      });
      expect(r.items[0].risk, RiskState.pass);
      expect(r.items[0].changePct24h, '1');
      expect(r.items[1].risk, RiskState.warn);
      expect(r.verdictNote, 'n');
    });
    test('timeline item keeps impact and evidence', () {
      final i = TimelineItem.fromJson(const {
        'time': '2026-10-28T09:10:00Z',
        'kind': 'hypothesis',
        'title': 't',
        'possible_impact': 'maybe',
        'evidence_ids': ['x'],
      });
      expect(i.kind, EvidenceKind.hypothesis);
      expect(i.possibleImpact, 'maybe');
      expect(i.evidenceIds, ['x']);
    });
  });

  group('Account, watchlist, alerts, shadow, briefing', () {
    test('Me and auth', () {
      final me = Me.fromJson(const {'id': '1', 'wallets': {'watch_only': 2, 'connected': 0}});
      expect(me.watchOnlyWallets, 2);
      expect(me.connectedWallets, 0);
      final a = AuthResult.fromJson(const {
        'access_token': 'tok',
        'expires_at': '2026-10-29T00:00:00Z',
        'user': {'id': 'u', 'email': 'e', 'name': 'n'},
      });
      expect(a.accessToken, 'tok');
      expect(a.user.email, 'e');
    });
    test('watchlist status falls back to unknown', () {
      final w = WatchlistItem.fromJson(const {
        'id': 'w1',
        'wallet': {'chain': 'solana', 'address': 'A'},
        'status': 'brand_new',
      });
      expect(w.status, WatchStatus.unknown);
      expect(WatchStatus.parse('partial_monitoring'), WatchStatus.partialMonitoring);
    });
    test('alerts page', () {
      final p = AlertsPage.fromJson(const {
        'items': [
          {'id': 'a', 'type': 'wallet_buy', 'title': 't', 'body': 'b', 'evidence_tx': ['x']},
          {'id': 'b', 'type': 'new_type'},
        ],
        'next_cursor': 'c2',
      });
      expect(p.items[0].type, AlertType.walletBuy);
      expect(p.items[1].type, AlertType.unknown);
      expect(p.nextCursor, 'c2');
    });
    test('shadow request json matches contract', () {
      const req = ShadowRequest(
        chain: Chain.solana,
        address: 'A',
        budgetUsd: '100',
        sizeRule: 'fixed_usd',
        delaySeconds: 30,
        slippageBps: 100,
        exitRule: 'mirror_sells',
      );
      expect(req.toJson(), {
        'wallet': {'chain': 'solana', 'address': 'A'},
        'budget_usd': '100',
        'size_rule': 'fixed_usd',
        'delay_seconds': 30,
        'slippage_bps': 100,
        'exit_rule': 'mirror_sells',
      });
    });
    test('shadow results tolerate missing baseline and numeric strings', () {
      final r = ShadowResults.fromJson(const {
        'data_status': 'demo',
        'demo': true,
        'return_pct': '48.6',
        'trades': '12',
        'baseline': {'return_pct': 31},
      });
      expect(r.env.isDemo, isTrue);
      expect(r.trades, 12);
      expect(r.baseline['return_pct'], '31');
      expect(r.assumptions, isEmpty);
      expect(r.endingUsd, isNull);
    });
    test('briefing sections', () {
      final b = BriefingResponse.fromJson(const {
        'data_status': 'live',
        'sections': [
          {
            'title': 'T',
            'items': [
              {'text': 'x', 'link': '/market'},
              {'text': 'y'},
            ],
          }
        ],
      });
      expect(b.sections.single.items[0].link, '/market');
      expect(b.sections.single.items[1].link, isNull);
    });
  });
}
