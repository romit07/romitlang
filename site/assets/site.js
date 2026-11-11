(function () {
  const storageKey = "romit-theme";

  function systemDark() {
    return window.matchMedia("(prefers-color-scheme: dark)").matches;
  }

  function applyTheme(mode) {
    if (mode === "light" || mode === "dark") {
      document.documentElement.setAttribute("data-theme", mode);
    } else {
      document.documentElement.removeAttribute("data-theme");
    }
  }

  const saved = localStorage.getItem(storageKey);
  applyTheme(saved || "");

  const toggle = document.querySelector("[data-theme-toggle]");
  if (toggle) {
    toggle.addEventListener("click", () => {
      const current = document.documentElement.getAttribute("data-theme");
      const next =
        current === "dark" ? "light" : current === "light" ? (systemDark() ? "light" : "dark") : systemDark() ? "light" : "dark";
      if (next) {
        localStorage.setItem(storageKey, next);
        applyTheme(next);
      } else {
        localStorage.removeItem(storageKey);
        applyTheme("");
      }
      toggle.textContent = document.documentElement.getAttribute("data-theme") === "dark" ? "Light" : "Dark";
    });
    toggle.textContent =
      (document.documentElement.getAttribute("data-theme") || (systemDark() ? "dark" : "light")) === "dark"
        ? "Light"
        : "Dark";
  }

  document.querySelectorAll("[data-install-tabs]").forEach((root) => {
    const panels = root.querySelectorAll("[data-install-panel]");
    const tabs = root.querySelectorAll("[data-install-tab]");
    tabs.forEach((tab) => {
      tab.addEventListener("click", () => {
        const id = tab.getAttribute("data-install-tab");
        tabs.forEach((t) => t.setAttribute("aria-selected", t === tab ? "true" : "false"));
        panels.forEach((p) => {
          p.hidden = p.getAttribute("data-install-panel") !== id;
        });
      });
    });
  });

  document.querySelectorAll("[data-copy-target]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const sel = btn.getAttribute("data-copy-target");
      const el = document.querySelector(sel);
      if (!el) return;
      const text = el.textContent || "";
      try {
        await navigator.clipboard.writeText(text.trim());
        const prev = btn.textContent;
        btn.textContent = "Copied";
        setTimeout(() => {
          btn.textContent = prev;
        }, 1200);
      } catch (_) {
        btn.textContent = "Copy failed";
      }
    });
  });
})();
