import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../api/models.dart';
import '../core/format.dart';
import '../core/tokens.dart';
import 'common.dart';

class MoverTile extends StatelessWidget {
  const MoverTile(this.item, {super.key, this.showRank = true});
  final MoverItem item;
  final bool showRank;

  @override
  Widget build(BuildContext context) {
    final change = parseDecimal(item.changePct);
    final label = '${item.token.displayName}, ${signedPercent(change)}, '
        '${item.reasonLabel}, volume ${compactUsdString(item.volumeUsd)}, '
        'risk check ${riskWord(item.risk)}';
    return TraceCard(
      onTap: () => context.push(item.token.path),
      semanticsLabel: label,
      child: Row(
        children: [
          if (showRank)
            SizedBox(
              width: 24,
              child: Text('${item.rank}', style: const TextStyle(color: TraceColors.textMuted)),
            ),
          TokenAvatar(item.token.displayName),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(item.token.displayName,
                    style: const TextStyle(fontWeight: FontWeight.w800, fontSize: 16)),
                Muted(item.reasonLabel, size: 12),
                const SizedBox(height: 4),
                RiskPill(item.risk),
              ],
            ),
          ),
          const SizedBox(width: 8),
          Column(
            crossAxisAlignment: CrossAxisAlignment.end,
            children: [
              Sparkline(item.sparkline),
              const SizedBox(height: 4),
              Text(signedPercent(change),
                  style: TextStyle(fontWeight: FontWeight.w800, color: changeColor(change))),
              Muted(compactUsdString(item.volumeUsd), size: 12),
            ],
          ),
        ],
      ),
    );
  }
}

String decisionText(WalletDecision d) => switch (d) {
      WalletDecision.eligible => 'Meets TRACE criteria for shadowing',
      WalletDecision.watchOnly => 'Watch only',
      WalletDecision.insufficientEvidence => 'Not enough history yet',
      WalletDecision.rejected => 'Does not meet TRACE criteria',
      WalletDecision.unknown => 'Status unknown',
    };

String confidenceText(PnlConfidence c) => switch (c) {
      PnlConfidence.high => 'High confidence',
      PnlConfidence.medium => 'Medium confidence',
      PnlConfidence.low => 'Low confidence',
      PnlConfidence.insufficient => 'Insufficient data',
    };

Color confidenceColor(PnlConfidence c) => switch (c) {
      PnlConfidence.high => TraceColors.positive,
      PnlConfidence.medium => TraceColors.info,
      PnlConfidence.low => TraceColors.warning,
      PnlConfidence.insufficient => TraceColors.textMuted,
    };

class WalletTile extends StatelessWidget {
  const WalletTile(this.item, {super.key, this.onTap});
  final WalletSummary item;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final ret = item.realizedReturnPct == null
        ? 'Not available'
        : signedPercentString(item.realizedReturnPct, decimals: 0);
    final win = item.winRatePct == null ? 'Not available' : plainPercentString(item.winRatePct, decimals: 0);
    return TraceCard(
      onTap: onTap ?? () => context.push(item.wallet.path, extra: item),
      semanticsLabel: '${item.wallet.displayName} on ${item.wallet.chain.label}. '
          'Past realized return $ret, win rate $win, ${item.sampleSize} trades. '
          '${confidenceText(item.pnlConfidence)}. ${decisionText(item.decision)}.',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(children: [
            const Icon(Icons.account_circle_outlined, color: TraceColors.emerald),
            const SizedBox(width: 8),
            Expanded(
              child: Text(item.wallet.displayName,
                  style: const TextStyle(fontWeight: FontWeight.w800, fontSize: 16)),
            ),
            Muted(item.wallet.chain.label, size: 12),
          ]),
          const SizedBox(height: 4),
          Muted(shortAddress(item.wallet.address), size: 12),
          if (item.tags.isNotEmpty) ...[
            const SizedBox(height: 8),
            Wrap(spacing: 6, runSpacing: 6, children: [for (final t in item.tags) Pill(t)]),
          ],
          const SizedBox(height: 8),
          Wrap(spacing: 16, runSpacing: 4, children: [
            Text('Past return: $ret', style: const TextStyle(fontWeight: FontWeight.w600)),
            Text('Win rate: $win'),
            Text('${item.sampleSize} trades'),
          ]),
          const SizedBox(height: 8),
          Wrap(spacing: 8, runSpacing: 4, crossAxisAlignment: WrapCrossAlignment.center, children: [
            Pill(confidenceText(item.pnlConfidence), color: confidenceColor(item.pnlConfidence)),
            Muted('Coverage ${coverageText(item.coverageRatio)}', size: 12),
          ]),
          const SizedBox(height: 4),
          Muted(decisionText(item.decision), size: 12),
        ],
      ),
    );
  }
}
