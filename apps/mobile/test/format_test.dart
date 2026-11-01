import 'package:flutter_test/flutter_test.dart';
import 'package:trace_mobile/core/format.dart';

void main() {
  group('compactUsd', () {
    test('scales', () {
      expect(compactUsdString('12400000'), '\$12.4M');
      expect(compactUsdString('127000'), '\$127K');
      expect(compactUsdString('8400000'), '\$8.4M');
      expect(compactUsdString('3100000'), '\$3.1M');
      expect(compactUsdString('2500000000'), '\$2.5B');
      expect(compactUsdString('25000000000000'), '\$25T');
    });
    test('small and mid values', () {
      expect(compactUsdString('2840.25'), '\$2,840.25');
      expect(compactUsdString('0.0084'), '\$0.0084');
      expect(compactUsdString('0.000012'), '\$0.000012');
      expect(compactUsdString('0'), '\$0.00');
      expect(compactUsdString('148.6'), '\$148.60');
    });
    test('negative and missing', () {
      expect(compactUsdString('-45000'), '-\$45K');
      expect(compactUsdString(null), 'Unavailable');
      expect(compactUsdString('abc'), 'Unavailable');
      expect(compactUsdString('12400000'), '\$12.4M');
      expect(compactUsdString('9007199254740993.125'), '\$9007T');
    });
  });

  group('signedPercent', () {
    test('adds sign', () {
      expect(signedPercent(42.7), '+42.7%');
      expect(signedPercent(-3.24), '-3.2%');
      expect(signedPercent(0), '0.0%');
      expect(signedPercent(-0.01), '0.0%');
      expect(signedPercent(1324, decimals: 0), '+1324%');
      expect(signedPercent(null), 'Unavailable');
      expect(signedPercentString('48.6'), '+48.6%');
    });
    test('plain percent and coverage', () {
      expect(plainPercentString('78', decimals: 0), '78%');
      expect(plainPercentString('66.7'), '66.7%');
      expect(coverageText('0.82'), '82%');
      expect(coverageText(null), 'Unavailable');
    });
  });

  group('relativeTime', () {
    final now = DateTime.utc(2026, 10, 28, 12, 0, 0);
    test('buckets', () {
      expect(relativeTime(now.subtract(const Duration(seconds: 10)), now: now),
          'just now');
      expect(relativeTime(now.subtract(const Duration(minutes: 14)), now: now),
          '14m ago');
      expect(relativeTime(now.subtract(const Duration(hours: 3)), now: now),
          '3h ago');
      expect(relativeTime(now.subtract(const Duration(days: 2)), now: now),
          '2d ago');
      expect(relativeTime(now.subtract(const Duration(days: 65)), now: now),
          '2mo ago');
      expect(
          relativeTime(now.add(const Duration(minutes: 5)), now: now), 'in 5m');
      expect(relativeTime(null, now: now), 'Unavailable');
    });
  });

  group('shortAddress and counts', () {
    test('shortens long addresses only', () {
      expect(shortAddress('7xAq3Fz9wLm2Rt8YbKd4VnHc6PjUe5SgXa1NoWi9kP'),
          '7xAq...i9kP');
      expect(shortAddress('abc'), 'abc');
      expect(shortAddress('0x1234567890abcdef', head: 6, tail: 4),
          '0x1234...cdef');
    });
    test('compactCount', () {
      expect(compactCount(432), '432');
      expect(compactCount(12480), '12.5K');
      expect(compactCount(1200), '1,200');
      expect(compactCount(null), 'Unavailable');
    });
  });
}
