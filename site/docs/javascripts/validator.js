(function () {
  "use strict";

  var KNOWN_RECIPE_TYPES = [
    "shell", "rust-bin", "rust-lib", "rust-test",
    "go-bin", "go-test",
    "c-bin", "c-lib",
    "cpp-bin", "cpp-lib",
    "ts-bin", "ts-check", "ts-lint", "npm",
    "py-bin", "py-test", "py-lint",
    "zig-bin", "zig-test",
    "java-bin", "java-test",
    "kotlin-bin", "kotlin-test",
    "swift-bin", "swift-test",
    "dotnet-build", "dotnet-test", "dotnet-publish",
    "ruby-bin", "ruby-test",
    "dart-bin", "dart-test",
    "elixir-build", "elixir-test",
    "flutter-build", "flutter-test",
    "nim-bin", "nim-test",
    "docker-build", "docker-push",
    "precompiled-bin", "bridge", "plugin",
  ];

  var REQUIRED_FIELDS = {
    shell: ["script"],
    "rust-bin": ["package"],
    "rust-lib": ["package"],
    "go-bin": ["package"],
    "c-bin": ["sources"],
    "cpp-bin": ["sources"],
    "ts-bin": ["package"],
    "py-bin": ["package"],
    "zig-bin": ["package"],
    "java-bin": ["package"],
    "kotlin-bin": ["package"],
    "swift-bin": ["package"],
    "dotnet-build": ["package"],
    "ruby-bin": ["package"],
    "dart-bin": ["package"],
    "elixir-build": ["package"],
    "flutter-build": ["package"],
    "nim-bin": ["package"],
    "docker-build": ["image"],
    "docker-push": ["image"],
    "precompiled-bin": ["url"],
    bridge: ["script"],
    plugin: ["plugin"],
  };

  function parseToml(input) {
    var result = { sections: {}, errors: [] };
    var lines = input.split("\n");
    var currentSection = null;
    var currentSubKey = null;
    var inArray = false;
    var i;

    for (i = 0; i < lines.length; i++) {
      var line = lines[i];
      var trimmed = line.trim();

      if (trimmed === "" || trimmed[0] === "#") continue;

      if (inArray) {
        if (trimmed[trimmed.length - 1] === "]") {
          inArray = false;
        }
        continue;
      }

      var sectionMatch = trimmed.match(/^\[([^\]]+)\]$/);
      if (sectionMatch) {
        currentSection = sectionMatch[1].trim();
        currentSubKey = null;
        result.sections[currentSection] = result.sections[currentSection] || {};
        continue;
      }

      var kvMatch = trimmed.match(/^([^=]+)=(.*)$/);
      if (kvMatch) {
        var key = kvMatch[1].trim();
        var val = kvMatch[2].trim();

        if (val[0] === "[" && val[val.length - 1] !== "]") {
          inArray = true;
        }

        if (currentSection) {
          result.sections[currentSection][key] = val;
        }
        continue;
      }

      if (trimmed.length > 0) {
        result.errors.push({ line: i + 1, message: "Cannot parse: " + trimmed.substring(0, 40) });
      }
    }

    return result;
  }

  function validate(parsed) {
    var warnings = [];
    var errors = parsed.errors.slice();

    if (!parsed.sections["project"]) {
      errors.push({ line: 0, message: "Missing [project] section" });
    } else {
      if (!parsed.sections["project"]["name"]) {
        errors.push({ line: 0, message: "Missing project name in [project]" });
      }
    }

    Object.keys(parsed.sections).forEach(function (section) {
      var recipeMatch = section.match(/^recipe\.([^.]+)$/);
      if (!recipeMatch) return;

      var recipeName = recipeMatch[1];
      var fields = parsed.sections[section];
      var typeVal = (fields["type"] || "").replace(/"/g, "");

      if (!fields["type"]) {
        errors.push({ line: 0, message: "Recipe '" + recipeName + "' missing type field" });
        return;
      }

      if (KNOWN_RECIPE_TYPES.indexOf(typeVal) === -1) {
        warnings.push({ line: 0, message: "Recipe '" + recipeName + "' has unknown type '" + typeVal + "'" });
      }

      var reqs = REQUIRED_FIELDS[typeVal];
      if (reqs) {
        reqs.forEach(function (req) {
          if (!fields[req]) {
            errors.push({ line: 0, message: "Recipe '" + recipeName + "' (" + typeVal + ") missing required field: " + req });
          }
        });
      }

      if (fields["cache"]) {
        var cacheVal = (fields["cache"] || "").replace(/"/g, "");
        if (["hash", "mtime", "none"].indexOf(cacheVal) === -1) {
          warnings.push({ line: 0, message: "Recipe '" + recipeName + "' has unknown cache strategy '" + cacheVal + "'" });
        }
      }
    });

    return { errors: errors, warnings: warnings };
  }

  function createValidator(id) {
    var container = document.getElementById(id);
    if (!container) return;

    var wrapper = document.createElement("div");
    wrapper.className = "must-validator";

    var textarea = document.createElement("textarea");
    textarea.spellcheck = false;
    textarea.placeholder = "Paste your Mustfile.toml here to validate...";

    var status = document.createElement("div");
    status.className = "must-validator-status must-validator-status--empty";
    status.textContent = "Paste TOML to validate";

    var debounceTimer = null;

    textarea.addEventListener("input", function () {
      clearTimeout(debounceTimer);
      debounceTimer = setTimeout(function () {
        var val = textarea.value.trim();
        if (!val) {
          status.className = "must-validator-status must-validator-status--empty";
          status.textContent = "Paste TOML to validate";
          return;
        }

        var parsed = parseToml(val);
        var result = validate(parsed);

        if (result.errors.length === 0 && result.warnings.length === 0) {
          status.className = "must-validator-status must-validator-status--valid";
          status.textContent = "\u2713 Valid Mustfile.toml";
        } else if (result.errors.length > 0) {
          status.className = "must-validator-status must-validator-status--invalid";
          status.textContent = "\u2717 " + result.errors.length + " error" + (result.errors.length > 1 ? "s" : "");
          if (result.errors.length <= 3) {
            status.textContent += " \u2014 " + result.errors.map(function (e) { return e.message; }).join("; ");
          }
        } else {
          status.className = "must-validator-status must-validator-status--valid";
          status.textContent = "\u2713 Valid (with " + result.warnings.length + " warning" + (result.warnings.length > 1 ? "s" : "") + ")";
          if (result.warnings.length <= 2) {
            status.textContent += " \u2014 " + result.warnings.map(function (w) { return w.message; }).join("; ");
          }
        }
      }, 300);
    });

    wrapper.appendChild(textarea);
    wrapper.appendChild(status);
    container.appendChild(wrapper);
  }

  function init() {
    document.querySelectorAll("[data-must-validator]").forEach(function (el) {
      createValidator(el.id);
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
