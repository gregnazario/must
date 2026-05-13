import 'package:test/test.dart';
import 'package:my_dart_package/my_dart_package.dart';

void main() {
  test('greet', () {
    expect(greet('world'), equals('Hello, world!'));
  });

  test('add', () {
    expect(add(2, 3), equals(5));
  });
}
