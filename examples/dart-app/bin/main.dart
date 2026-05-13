import 'package:dart_app/dart_app.dart';

void main(List<String> arguments) {
  print(greet(arguments.isNotEmpty ? arguments.first : 'world'));
}
