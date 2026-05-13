# Dart Recipes

must provides two Dart recipe types: `dart-bin` and `dart-test`.

## Quick start

### Project structure

```
myapp/
├── Mustfile.toml
├── pubspec.yaml
├── bin/
│   └── myapp.dart
└── test/
    └── myapp_test.dart
```

### Source files

`pubspec.yaml`:

```yaml
name: myapp
environment:
  sdk: ^3.5.0

dependencies:
  test: ^1.25.0

executables:
  myapp: myapp
```

`bin/myapp.dart`:

```dart
String greet(String name) => 'Hello, $name!';

void main() {
  print(greet('world'));
}
```

`test/myapp_test.dart`:

```dart
import 'package:test/test.dart';
import 'package:myapp/myapp.dart';

void main() {
  test('greet returns greeting', () {
    expect(greet('world'), equals('Hello, world!'));
  });
}
```

`Mustfile.toml`:

```toml
[project]
name = "myapp"

[recipe.build]
type    = "dart-bin"
package = "bin/myapp.dart"

[recipe.test]
type    = "dart-test"
deps    = ["build"]
```

### Build and test

```
$ must build
[1/1] build (dart-bin)  dart compile exe bin/myapp.dart
Generated: myapp.exe

$ must test
[1/2] build (dart-bin)  cached
[2/2] test (dart-test)  dart test
00:00 +1: All tests passed!
```

## `dart-bin` — Compile a binary

```toml
[recipe.build]
type    = "dart-bin"
package = "bin/main.dart"
```

Runs: `dart compile exe bin/main.dart`

Cache key includes: package, env vars.

## `dart-test` — Run tests

```toml
[recipe.test]
type    = "dart-test"
package = "test/"
deps    = ["build"]
```

Runs: `dart test` (in package directory)

Cache strategy: `never` (test results should always be fresh).

## Fields

| Field | Type | Applies to | Description |
|-------|------|-----------|-------------|
| `package` | string | bin | Dart entry point file |
| `package` | string | test | Test directory path |
| `deps` | string[] | all | Dependencies |
| `env` | map | all | Environment variables |

All recipes also support these common fields: `deps`, `env`, `cache` (`"hash"` / `"mtime"` / `"none"`), `phony` (always re-run).

## Examples

```toml
[project]
name = "my-dart-project"

[recipe.build]
type    = "dart-bin"
package = "bin/main.dart"

[recipe.test]
type    = "dart-test"
package = "."
deps    = ["build"]
```

## See also

- [Caching](../guide/caching.md) — how cache keys are computed
- [Config Reference](../guide/config-reference.md) — full field reference
- [Flutter Recipes](flutter.md) — Flutter build and test
