"use strict";
(() => {
    const KNOWN_RECIPE_TYPES = [
        "shell",
        "rust-bin",
        "rust-lib",
        "rust-test",
        "go-bin",
        "go-test",
        "c-bin",
        "c-lib",
        "cpp-bin",
        "cpp-lib",
        "ts-bin",
        "ts-check",
        "ts-lint",
        "npm",
        "py-bin",
        "py-test",
        "py-lint",
        "zig-bin",
        "zig-test",
        "java-bin",
        "java-test",
        "kotlin-bin",
        "kotlin-test",
        "swift-bin",
        "swift-test",
        "dotnet-build",
        "dotnet-test",
        "dotnet-publish",
        "ruby-bin",
        "ruby-test",
        "dart-bin",
        "dart-test",
        "elixir-build",
        "elixir-test",
        "flutter-build",
        "flutter-test",
        "nim-bin",
        "nim-test",
        "docker-build",
        "docker-push",
        "precompiled-bin",
        "bridge",
        "plugin",
    ];
    const REQUIRED_FIELDS = {
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
    const VALID_CACHE_STRATEGIES = ["hash", "mtime", "none"];
    function unquote(s) {
        return s.replace(/"/g, "");
    }
    function parseToml(input) {
        const result = { sections: {}, errors: [] };
        const lines = input.split("\n");
        let currentSection = null;
        let inArray = false;
        for (let i = 0; i < lines.length; i++) {
            const trimmed = lines[i].trim();
            if (trimmed === "" || trimmed[0] === "#")
                continue;
            if (inArray) {
                if (trimmed[trimmed.length - 1] === "]") {
                    inArray = false;
                }
                continue;
            }
            const sectionMatch = trimmed.match(/^\[([^\]]+)\]$/);
            if (sectionMatch) {
                currentSection = sectionMatch[1].trim();
                result.sections[currentSection] = result.sections[currentSection] || {};
                continue;
            }
            const kvMatch = trimmed.match(/^([^=]+)=(.*)$/);
            if (kvMatch && currentSection) {
                const key = kvMatch[1].trim();
                const val = kvMatch[2].trim();
                if (val[0] === "[" && val[val.length - 1] !== "]") {
                    inArray = true;
                }
                result.sections[currentSection][key] = val;
                continue;
            }
            if (trimmed.length > 0) {
                result.errors.push({
                    line: i + 1,
                    message: `Cannot parse: ${trimmed.substring(0, 40)}`,
                });
            }
        }
        return result;
    }
    function validateToml(parsed) {
        const warnings = [];
        const errors = [...parsed.errors];
        const project = parsed.sections.project;
        if (!project) {
            errors.push({ line: 0, message: "Missing [project] section" });
        }
        else if (!project.name) {
            errors.push({ line: 0, message: "Missing project name in [project]" });
        }
        for (const section of Object.keys(parsed.sections)) {
            const recipeMatch = section.match(/^recipe\.([^.]+)$/);
            if (!recipeMatch)
                continue;
            const recipeName = recipeMatch[1];
            const fields = parsed.sections[section];
            const typeVal = unquote(fields.type || "");
            if (!fields.type) {
                errors.push({ line: 0, message: `Recipe '${recipeName}' missing type field` });
                continue;
            }
            if (!KNOWN_RECIPE_TYPES.includes(typeVal)) {
                warnings.push({ line: 0, message: `Recipe '${recipeName}' has unknown type '${typeVal}'` });
            }
            const reqs = REQUIRED_FIELDS[typeVal];
            if (reqs) {
                for (const req of reqs) {
                    if (!fields[req]) {
                        errors.push({
                            line: 0,
                            message: `Recipe '${recipeName}' (${typeVal}) missing required field: ${req}`,
                        });
                    }
                }
            }
            if (fields.cache) {
                const cacheVal = unquote(fields.cache);
                if (!VALID_CACHE_STRATEGIES.includes(cacheVal)) {
                    warnings.push({
                        line: 0,
                        message: `Recipe '${recipeName}' has unknown cache strategy '${cacheVal}'`,
                    });
                }
            }
        }
        return { errors, warnings };
    }
    function pluralize(count, singular) {
        return count === 1 ? singular : `${singular}s`;
    }
    function formatMessages(items, max) {
        if (items.length <= max) {
            return items.map((e) => e.message).join("; ");
        }
        return "";
    }
    function createValidator(container) {
        const wrapper = document.createElement("div");
        wrapper.className = "must-validator";
        const textarea = document.createElement("textarea");
        textarea.spellcheck = false;
        textarea.placeholder = "Paste your Mustfile.toml here to validate...";
        const status = document.createElement("div");
        status.className = "must-validator-status must-validator-status--empty";
        status.textContent = "Paste TOML to validate";
        let debounceTimer = null;
        textarea.addEventListener("input", () => {
            if (debounceTimer)
                clearTimeout(debounceTimer);
            debounceTimer = setTimeout(() => {
                const val = textarea.value.trim();
                if (!val) {
                    status.className = "must-validator-status must-validator-status--empty";
                    status.textContent = "Paste TOML to validate";
                    return;
                }
                const parsed = parseToml(val);
                const result = validateToml(parsed);
                if (result.errors.length === 0 && result.warnings.length === 0) {
                    status.className = "must-validator-status must-validator-status--valid";
                    status.textContent = "\u2713 Valid Mustfile.toml";
                }
                else if (result.errors.length > 0) {
                    status.className = "must-validator-status must-validator-status--invalid";
                    let text = `\u2717 ${result.errors.length} ${pluralize(result.errors.length, "error")}`;
                    const details = formatMessages(result.errors, 3);
                    if (details)
                        text += ` \u2014 ${details}`;
                    status.textContent = text;
                }
                else {
                    status.className = "must-validator-status must-validator-status--valid";
                    let text = `\u2713 Valid (with ${result.warnings.length} ${pluralize(result.warnings.length, "warning")})`;
                    const details = formatMessages(result.warnings, 2);
                    if (details)
                        text += ` \u2014 ${details}`;
                    status.textContent = text;
                }
            }, 300);
        });
        wrapper.appendChild(textarea);
        wrapper.appendChild(status);
        container.appendChild(wrapper);
    }
    function init() {
        document.querySelectorAll("[data-must-validator]").forEach((el) => {
            createValidator(el);
        });
    }
    if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", init);
    }
    else {
        init();
    }
})();
