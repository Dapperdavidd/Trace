import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:trace_mobile/api/demo_api.dart';
import 'package:trace_mobile/api/models.dart';
import 'package:trace_mobile/app.dart';
import 'package:trace_mobile/state/session.dart';

void main() {
  final api = DemoApi(clock: () => DateTime.utc(2026, 10, 28, 12));
  final neura = DemoApi.addressOf('NEURA');

  group('DemoApi fixtures are always flagged demo', () {
    test('every analytic response carries demo status', () async {
      final envs = <Envelope>[
        (await api.movers()).env,
        (await api.discoveryWallets()).env,
        (await api.wallet(Chain.solana, DemoApi.demoWalletAddress)).env,
        (await api.walletTrades(Chain.solana, DemoApi.demoWalletAddress)).env,
        (await api.walletMetrics(Chain.solana, DemoApi.demoWalletAddress)).env,
        (await api.walletDna(Chain.solana, DemoApi.demoWalletAddress)).env,
        (await api.token(Chain.solana, neura)).env,
        (await api.tokenTimeline(Chain.solana, neura)).env,
        (await api.compare(TokenRef(chain: Chain.solana, address: neura), TokenRef(chain: Chain.solana, address: DemoApi.addressOf('SOLX')))).env,
        (await api.shadowResults('demo-shadow-1')).env,
        (await api.briefings()).env,
      ];
      for (final e in envs) {
        expect(e.isDemo, isTrue);
        expect(e.status, DataStatus.demo);
      }
    });
    test('movers match the design numbers and have eight rows', () async {
      final m = (await api.movers()).items;
      expect(m, hasLength(8));
      expect(m.first.token.symbol, 'NEURA');
      expect(m.first.changePct, '42.7');
      expect(m.first.volumeUsd, '12400000');
    });
    test('insufficient_history is exercised by the thin demo wallet', () async {
      final d = await api.walletDna(Chain.solana, DemoApi.demoThinWalletAddress);
      expect(d.insufficientHistory, isTrue);
      expect(d.traits, isEmpty);
      final ok = await api.walletDna(Chain.solana, DemoApi.demoWalletAddress);
      expect(ok.insufficientHistory, isFalse);
      expect(ok.traits, isNotEmpty);
    });
    test('why moving separates facts from hypotheses', () async {
      final t = await api.token(Chain.solana, neura);
      expect(t.whyMoving.where((w) => w.kind == EvidenceKind.fact), isNotEmpty);
      expect(t.whyMoving.where((w) => w.kind == EvidenceKind.hypothesis), isNotEmpty);
      for (final w in t.whyMoving.where((w) => w.kind == EvidenceKind.fact)) {
        expect(w.evidenceIds, isNotEmpty);
      }
    });
    test('watchlist add is idempotent and removable', () async {
      final d = DemoApi();
      final before = (await d.watchlists()).length;
      await d.addWatch(Chain.solana, DemoApi.addressOf('FROG'), alias: 'x');
      await d.addWatch(Chain.solana, DemoApi.addressOf('FROG'));
      final after = await d.watchlists();
      expect(after.length, before + 1);
      await d.removeWatch(after.last.id);
      expect((await d.watchlists()).length, before);
    });
    test('createShadow returns a demo id', () async {
      final id = await api.createShadow(const ShadowRequest(
        chain: Chain.solana,
        address: 'A',
        budgetUsd: '100',
        sizeRule: 'fixed_usd',
        delaySeconds: 30,
        slippageBps: 100,
        exitRule: 'mirror_sells',
      ));
      expect(id, startsWith('demo-'));
    });
  });

  group('routing decision', () {
    test('loading goes to splash', () {
      expect(redirectFor(SessionPhase.loading, '/home'), '/splash');
      expect(redirectFor(SessionPhase.loading, '/splash'), isNull);
    });
    test('signed out users are sent to welcome', () {
      expect(redirectFor(SessionPhase.signedOut, '/home'), '/welcome');
      expect(redirectFor(SessionPhase.signedOut, '/welcome'), isNull);
    });
    test('explorers and signed in users skip welcome', () {
      for (final p in [SessionPhase.explorer, SessionPhase.signedIn]) {
        expect(redirectFor(p, '/welcome'), '/home');
        expect(redirectFor(p, '/splash'), '/home');
        expect(redirectFor(p, '/market'), isNull);
      }
    });
  });

  group('repository hygiene', () {
    final emDash = String.fromCharCode(0x2014);
    final enDash = String.fromCharCode(0x2013);

    Iterable<File> sources() sync* {
      for (final dir in ['lib', 'test']) {
        final d = Directory(dir);
        if (!d.existsSync()) continue;
        for (final f in d.listSync(recursive: true)) {
          if (f is File && f.path.endsWith('.dart')) yield f;
        }
      }
      for (final name in ['README.md', 'pubspec.yaml', 'analysis_options.yaml']) {
        final f = File(name);
        if (f.existsSync()) yield f;
      }
    }

    test('no em dash or en dash characters anywhere', () {
      final offenders = <String>[];
      for (final f in sources()) {
        final text = f.readAsStringSync();
        if (text.contains(emDash) || text.contains(enDash)) offenders.add(f.path);
      }
      expect(offenders, isEmpty);
    });

    test('no profit guarantee or win-probability language in UI copy', () {
      final banned = RegExp(r'guaranteed (profit|return)|risk[- ]free|win probability|will (go|moon)', caseSensitive: false);
      final offenders = <String>[];
      for (final f in Directory('lib').listSync(recursive: true).whereType<File>()) {
        if (!f.path.endsWith('.dart')) continue;
        if (banned.hasMatch(f.readAsStringSync())) offenders.add(f.path);
      }
      expect(offenders, isEmpty);
    });
  });
}
