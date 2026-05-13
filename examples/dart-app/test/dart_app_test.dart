import 'package:test/test.dart';
import 'package:dart_app/dart_app.dart';

void main() {
  test('greet returns greeting', () {
    expect(greet('world'), equals('Hello, world!'));
  });

  test('add returns sum', () {
    expect(add(2, 3), equals(5));
  });
}
