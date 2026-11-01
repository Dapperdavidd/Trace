/// Display formatters. Display only: the client never computes authoritative PnL.
const String kNotAvailable = 'Unavailable';

double? parseDecimal(String? s) {
  if (s == null) return null;
  return double.tryParse(s.trim());
}

String _trimZeros(String s) {
  if (!s.contains('.')) return s;
  var out = s;
  while (out.endsWith('0')) {
    out = out.substring(0, out.length - 1);
  }
  if (out.endsWith('.')) out = out.substring(0, out.length - 1);
  return out;
}

String _withThousands(String intPart) {
  final buf = StringBuffer();
  for (var i = 0; i < intPart.length; i++) {
    if (i > 0 && (intPart.length - i) % 3 == 0) buf.write(',');
    buf.write(intPart[i]);
  }
  return buf.toString();
}

/// Compact USD from an API decimal string. Money is never converted to a float.
String compactUsdString(String? s) {
  if (s == null) return kNotAvailable;
  final match = RegExp(r'^([+-]?)(\d+)(?:\.(\d+))?$').firstMatch(s.trim());
  if (match == null) return kNotAvailable;
  final whole = match.group(2) ?? '0';
  final fraction = match.group(3) ?? '';
  final coefficient = BigInt.parse('$whole$fraction');
  final scale = fraction.length;
  final negative = match.group(1) == '-' && coefficient != BigInt.zero;
  final tenScale = BigInt.from(10).pow(scale);

  bool atLeast(BigInt value) => coefficient >= value * tenScale;
  BigInt rounded(BigInt divisor, int decimals) {
    final factor = BigInt.from(10).pow(decimals);
    final denominator = tenScale * divisor;
    return (coefficient * factor + denominator ~/ BigInt.two) ~/ denominator;
  }

  String render(BigInt value, int decimals,
      {bool trim = true, bool group = false}) {
    final raw = value.toString().padLeft(decimals + 1, '0');
    final integer =
        decimals == 0 ? raw : raw.substring(0, raw.length - decimals);
    var decimal = decimals == 0 ? '' : raw.substring(raw.length - decimals);
    if (trim) decimal = decimal.replaceFirst(RegExp(r'0+$'), '');
    final displayedInteger = group ? _withThousands(integer) : integer;
    return decimal.isEmpty ? displayedInteger : '$displayedInteger.$decimal';
  }

  final units = <(BigInt, String)>[
    (BigInt.from(10).pow(12), 'T'),
    (BigInt.from(10).pow(9), 'B'),
    (BigInt.from(10).pow(6), 'M'),
    (BigInt.from(10).pow(3), 'K'),
  ];
  final sign = negative ? '-' : '';
  for (final (size, suffix) in units) {
    if (suffix == 'K' && !atLeast(BigInt.from(10000))) continue;
    if (atLeast(size)) {
      final decimals = atLeast(size * BigInt.from(100))
          ? 0
          : (atLeast(size * BigInt.from(10)) ? 1 : 2);
      return '$sign\$${render(rounded(size, decimals), decimals)}$suffix';
    }
  }
  final decimals = coefficient == BigInt.zero
      ? 2
      : (atLeast(BigInt.one) ? 2 : scale.clamp(4, 8));
  return '$sign\$${render(rounded(BigInt.one, decimals), decimals, trim: !atLeast(BigInt.one) && coefficient != BigInt.zero, group: atLeast(BigInt.one))}';
}

/// Signed percent: +42.7%, -3.2%, 0.0%.
String signedPercent(double? v, {int decimals = 1}) {
  if (v == null || v.isNaN || v.isInfinite) return kNotAvailable;
  final fixed = v.abs().toStringAsFixed(decimals);
  final isZero = double.parse(fixed) == 0;
  if (isZero) return '$fixed%';
  return '${v < 0 ? '-' : '+'}$fixed%';
}

String signedPercentString(String? s, {int decimals = 1}) =>
    signedPercent(parseDecimal(s), decimals: decimals);

/// Unsigned percent: 78%, 66.7%.
String plainPercent(double? v, {int decimals = 1}) {
  if (v == null || v.isNaN || v.isInfinite) return kNotAvailable;
  return '${_trimZeros(v.toStringAsFixed(decimals))}%';
}

String plainPercentString(String? s, {int decimals = 1}) =>
    plainPercent(parseDecimal(s), decimals: decimals);

/// Coverage ratio string like "0.82" shown as "82%".
String coverageText(String? ratio) {
  final r = parseDecimal(ratio);
  if (r == null) return kNotAvailable;
  return plainPercent(r * 100, decimals: 0);
}

String compactCount(int? v) {
  if (v == null) return kNotAvailable;
  if (v >= 1000000) return '${_trimZeros((v / 1e6).toStringAsFixed(1))}M';
  if (v >= 10000) return '${_trimZeros((v / 1e3).toStringAsFixed(1))}K';
  return _withThousands(v.toString());
}

/// Relative time: just now, 14m ago, 3h ago, 1d ago, in 5m.
String relativeTime(DateTime? t, {DateTime? now}) {
  if (t == null) return kNotAvailable;
  final ref = (now ?? DateTime.now()).toUtc();
  final diff = ref.difference(t.toUtc());
  final future = diff.isNegative;
  final d = future ? -diff : diff;
  String unit;
  if (d.inSeconds < 45) {
    return 'just now';
  } else if (d.inMinutes < 60) {
    unit = '${d.inMinutes < 1 ? 1 : d.inMinutes}m';
  } else if (d.inHours < 24) {
    unit = '${d.inHours}h';
  } else if (d.inDays < 30) {
    unit = '${d.inDays}d';
  } else if (d.inDays < 365) {
    unit = '${d.inDays ~/ 30}mo';
  } else {
    unit = '${d.inDays ~/ 365}y';
  }
  return future ? 'in $unit' : '$unit ago';
}

/// 7xAq...9kPQ style short address.
String shortAddress(String address, {int head = 4, int tail = 4}) {
  final a = address.trim();
  if (a.length <= head + tail + 3) return a;
  return '${a.substring(0, head)}...${a.substring(a.length - tail)}';
}

/// HH:MM in UTC for timelines.
String clockUtc(DateTime? t) {
  if (t == null) return kNotAvailable;
  final u = t.toUtc();
  final h = u.hour.toString().padLeft(2, '0');
  final m = u.minute.toString().padLeft(2, '0');
  return '$h:$m UTC';
}
