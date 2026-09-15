/**
 * 右侧抽屉 enter/leave 钩子（与转换页版本选择器同一套实现）。
 *
 * 用 inline style + CSS transition 驱动，绕开 App.vue 全局
 * `.page-shell > * > *` 的 page-entry stagger（那条规则会锁 opacity）。
 * 侧栏需使用全局类 `sidebar-content`（或自带 equivalent 定位），
 * 且 `opacity: 1 !important`。
 */
export function useSidebarSlide(options?: { shadow?: string }) {
  const shadow = options?.shadow ?? "-12px 0 36px rgba(0, 0, 0, 0.08)";

  function prefersReducedMotion() {
    return document.body.classList.contains("motion-reduced");
  }

  function onBeforeEnter(el: Element) {
    const h = el as HTMLElement;
    h.style.transition = "none";
    h.style.transform = "translateX(100%)";
    h.style.boxShadow = "none";
    h.style.opacity = "1";
  }

  function onEnter(el: Element, done: () => void) {
    const h = el as HTMLElement;
    if (prefersReducedMotion()) {
      h.style.transition = "";
      h.style.transform = "translateX(0)";
      h.style.boxShadow = shadow;
      h.style.opacity = "1";
      done();
      return;
    }
    // 强制 reflow，让 transition:none 先 commit
    h.offsetHeight;
    requestAnimationFrame(() => {
      h.style.transition =
        "transform 320ms cubic-bezier(0.22, 1, 0.36, 1), box-shadow 320ms cubic-bezier(0.22, 1, 0.36, 1)";
      h.style.transform = "translateX(0)";
      h.style.boxShadow = shadow;
      setTimeout(done, 330);
    });
  }

  function onAfterEnter(el: Element) {
    const h = el as HTMLElement;
    h.style.transition = "";
    h.style.transform = "translateX(0)";
    h.style.boxShadow = shadow;
    h.style.opacity = "1";
  }

  function onBeforeLeave(el: Element) {
    const h = el as HTMLElement;
    if (prefersReducedMotion()) {
      h.style.transition = "";
      h.style.transform = "translateX(100%)";
      h.style.boxShadow = "none";
      return;
    }
    h.style.transition =
      "transform 240ms cubic-bezier(0.4, 0, 1, 1), box-shadow 240ms cubic-bezier(0.4, 0, 1, 1)";
    h.style.transform = "translateX(100%)";
    h.style.boxShadow = "none";
  }

  function onLeave(_el: Element, done: () => void) {
    setTimeout(done, prefersReducedMotion() ? 0 : 250);
  }

  function onAfterLeave(el: Element) {
    const h = el as HTMLElement;
    h.style.transition = "";
    h.style.transform = "";
    h.style.boxShadow = "";
    h.style.opacity = "";
  }

  return {
    onBeforeEnter,
    onEnter,
    onAfterEnter,
    onBeforeLeave,
    onLeave,
    onAfterLeave,
  };
}

