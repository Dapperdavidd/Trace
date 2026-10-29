import 'package:flutter_test/flutter_test.dart';
import 'package:trace_mobile/core/format.dart';

void main() {
  group('compactUsd', () {
    test('scales', () {
      expect(compactUsd(12400000), '\$12.4M');
      expect(compactUsd(127000), '\$127K');
      expect(compactUsd(8400000), '\$8.4M');
      expect(compactUsd(3100000), '\$3.1M');
      expect(compactUsd(2.5e9), '\$2.5B');
      expect(compactUsd(25e12), '\$25T');
    });
    test('small and mid values', () {
      expect(compactUsd(2840.25), '\$2,840.25');
      expect(compactUsd(0.0084), '\$0.0084');
      expect(compactUsd(0.000012), '\$0.000012');
      expect(compactUsd(0), '\$0.00');
      expect(compactUsd(148.6), '\$148.60');
    });
    test('negative and missing', () {
      expect(compactUsd(-45000), '-\$45K');
      expect(compactUsd(null), 'n/a');
      expect(compactUsd(double.nan), 'n/a');
      expect(compactUsdString('abc'), 'n/a');
      expect(compactUsdString('12400000'), '\$12.4M');
    });
  });

  group('signedPercent', () {
    test('adds sign', () {
      expect(signedPercent(42.7), '+42.7%');
      expect(signedPercent(-3.24), '-3.2%');
      expect(signedPercent(0), '0.0%');
      expect(signedPercent(-0.01), '0.0%');
      expect(signedPercent(1324, decimals: 0), '+1324%');
      expect(signedPercent(null), 'n/a');
      expect(signedPercentString('48.6'), '+48.6%');
    });
    test('plain percent and coverage', () {
      expect(plainPercentString('78', decimals: 0), '78%');
      expect(plainPercentString('66.7'), '66.7%');
      expect(coverageText('0.82'), '82%');
      expect(coverageText(null), 'n/a');
    });
  });

  group('relativeTime', () {
    final now = DateTime.utc(2026, 10, 28, 12, 0, 0);
    test('buckets', () {
      expect(relativeTime(now.subtract(const Duration(seconds: 10)), now: now), 'just now');
      expect(relativeTime(now.subtract(const Duration(minutes: 14)), now: now), '14m ago');
      expect(relativeTime(now.subtract(const Duration(hours: 3)), now: now), '3h ago');
      expect(relativeTime(now.subtract(const Duration(days: 2)), now: now), '2d ago');
      expect(relativeTime(now.subtract(const Duration(days: 65)), now: now), '2mo ago');
      expect(relativeTime(now.add(const Duration(minutes: 5)), now: now), 'in 5m');
      expect(relativeTime(null, now: now), 'n/a');
    });
  });

  group('shortAddress and counts', () {
    test('shortens long addresses only', () {
      expect(shortAddress('7xAq3Fz9wLm2Rt8YbKd4VnHc6PjUe5SgXa1NoWi9kP'), '7xAq...i9kP');
      expect(shortAddress('abc'), 'abc');
      expect(shortAddress('0x1234567890abcdef', head: 6, tail: 4), '0x1234...cdef');
    });
    test('compactCount', () {
      expect(compactCount(432), '432');
      expect(compactCount(12480), '12.5K');
      expect(compactCount(1200), '1,200');
      expect(compactCount(null), 'n/a');
    });
  });
}
