// Foray design preview — light interactivity only
(function () {
  const toc = document.querySelectorAll("nav.toc a[href^='#']");
  const sections = [...document.querySelectorAll("section.card[id]")];

  function setActive() {
    const y = window.scrollY + 96;
    let current = sections[0]?.id;
    for (const s of sections) {
      if (s.offsetTop <= y) current = s.id;
    }
    toc.forEach((a) => {
      const on = a.getAttribute("href") === "#" + current;
      a.style.color = on ? "var(--ink)" : "";
      a.style.borderColor = on ? "var(--line)" : "transparent";
    });
  }
  window.addEventListener("scroll", setActive, { passive: true });
  setActive();

  // Smooth scroll for TOC
  toc.forEach((a) => {
    a.addEventListener("click", (e) => {
      const id = a.getAttribute("href").slice(1);
      const el = document.getElementById(id);
      if (!el) return;
      e.preventDefault();
      window.scrollTo({ top: el.offsetTop - 72, behavior: "smooth" });
    });
  });

  // Demo: AI tier picker preview list
  const tierSelect = document.getElementById("tier-demo");
  const preview = document.getElementById("tier-preview");
  const TIERS = {
    0: ["（不发送任何内容）"],
    1: ["目录树（paths）", "扩展名统计"],
    2: ["+ pack.mcmeta 全文"],
    3: ["+ 勾选的 JSON 副本（≤20 × 64KB）"],
    4: ["+ 勾选的着色器源码（≤10 × 128KB）"],
    5: ["+ 贴图概括（尺寸/格式/均色/直方图摘要，非像素）"],
  };
  function renderTier(t) {
    const list = TIERS[t] || TIERS[1];
    preview.innerHTML = list.map((x) => `<li>${x}</li>`).join("");
  }
  if (tierSelect && preview) {
    tierSelect.addEventListener("change", () => renderTier(Number(tierSelect.value)));
    renderTier(1);
  }
})();
