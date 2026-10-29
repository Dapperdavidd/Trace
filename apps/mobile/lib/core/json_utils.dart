/// Tolerant JSON readers. They never throw on missing or wrongly typed fields.
Map<String, dynamic> asMap(Object? v) =>
    v is Map ? Map<String, dynamic>.from(v) : <String, dynamic>{};

List<Object?> asList(Object? v) => v is List ? List<Object?>.from(v) : const <Object?>[];

String? strOrNull(Object? v) {
  if (v == null) return null;
  if (v is String) return v;
  if (v is num || v is bool) return v.toString();
  return null;
}

String str(Object? v, [String fallback = '']) => strOrNull(v) ?? fallback;

int intOr(Object? v, [int fallback = 0]) => intOrNull(v) ?? fallback;

int? intOrNull(Object? v) {
  if (v is int) return v;
  if (v is num) return v.toInt();
  if (v is String) return int.tryParse(v.trim());
  return null;
}

double? doubleOrNull(Object? v) {
  if (v is num) return v.toDouble();
  if (v is String) return double.tryParse(v.trim());
  return null;
}

bool boolOr(Object? v, [bool fallback = false]) {
  if (v is bool) return v;
  if (v is String) {
    final s = v.toLowerCase();
    if (s == 'true') return true;
    if (s == 'false') return false;
  }
  return fallback;
}

DateTime? dateOrNull(Object? v) {
  final s = strOrNull(v);
  if (s == null || s.isEmpty) return null;
  return DateTime.tryParse(s);
}

List<String> strList(Object? v) =>
    asList(v).map(strOrNull).whereType<String>().toList();

List<double> doubleList(Object? v) =>
    asList(v).map(doubleOrNull).whereType<double>().toList();

List<T> mapList<T>(Object? v, T Function(Map<String, dynamic>) parse) => asList(v)
    .whereType<Map<dynamic, dynamic>>()
    .map((e) => parse(Map<String, dynamic>.from(e)))
    .toList();

Map<String, String> stringMap(Object? v) {
  final out = <String, String>{};
  asMap(v).forEach((k, val) {
    final s = strOrNull(val);
    if (s != null) out[k] = s;
  });
  return out;
}
