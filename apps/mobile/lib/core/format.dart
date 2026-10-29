/// Display formatters. Display only: the client never computes authoritative PnL.
const String kNotAvailable = 'n/a';

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

/// Compact USD: $12.4M, $127K, $2,840.25, $0.0084. Null gives "n/a".
String compactUsd(double? v) {
  if (v == null || v.isNaN || v.isInfinite) return kNotAvailable;
  final neg = v < 0;
  final a = v.abs();
  String body;
  if (a >= 1e12) {
    body = '${_trimZeros((a / 1e12).toStringAsFixed(1))}T';
  } else if (a >= 1e9) {
    body = '${_trimZeros((a / 1e9).toStringAsFixed(1))}B';
  } else if (a >= 1e6) {
    body = '${_trimZeros((a / 1e6).toStringAsFixed(1))}M';
  } else if (a >= 1e4) {
    body = '${_trimZeros((a / 1e3).toStringAsFixed(1))}K';
  } else if (a >= 1) {
    final fixed = a.toStringAsFixed(2);
    final parts = fixed.split('.');
    body = '${_withThousands(parts[0])}.${parts[1]}';
  } else if (a == 0) {
    body = '0.00';
  } else {
    // Below $1: up to 6 decimals, but never fewer than 4.
    var small = a.toStringAsFixed(6);
    while (small.endsWith('0') && small.length - small.indexOf('.') - 1 > 4) {
      small = small.substring(0, small.length - 1);
    }
    body = small;
  }
  return '${neg ? '-' : ''}\$$body';
}

String compactUsdString(String? s) => compactUsd(parseDecimal(s));

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
