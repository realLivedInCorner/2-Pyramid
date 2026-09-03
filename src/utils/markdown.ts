// 极简 Markdown 渲染器 —— 用于更新日志（Release body）展示。
//
// 安全设计：输入视为不可信（GitHub Release body 由发布者填写）。
//   1. 全部文本先做 HTML 转义，再进行 Markdown 变换，任何原始 HTML
//      都只会显示为文本（不会被执行）；
//   2. 链接仅允许 http/https，且写入 href 前剔除引号/尖括号，
//      前端点击时通过插件打开外部浏览器（不在 WebView 内导航）；
//   3. 渲染结果通过 v-html 注入，但所有内容均经过上述转义。
//
// 支持的语法（覆盖更新日志常用子集）：
//   # / ## / ### / #### 标题、- / * / 1. 列表、> 引用、``` 代码块、
//   **加粗**、*斜体*、`行内代码`、[文本](https://链接)、--- 分隔线

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function inline(text: string): string {
  // 行内代码（内容已转义）
  let out = text.replace(/`([^`]+)`/g, (_m, c: string) => `<code>${c}</code>`);
  // 链接 [label](url)：仅 http/https，href 中剔除危险字符
  out = out.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (_m, label: string, url: string) => {
    if (!/^https?:\/\//i.test(url)) return label;
    const clean = url.replace(/["'<>]/g, "");
    return `<a href="${clean}" data-ext-link="1">${label}</a>`;
  });
  // 加粗 / 斜体
  out = out.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  out = out.replace(/\*([^*]+)\*/g, "<em>$1</em>");
  return out;
}

export function renderMarkdown(src: string): string {
  const lines = (src || "").replace(/\r\n/g, "\n").split("\n");
  const html: string[] = [];
  let para: string[] = [];
  let listType: "ul" | "ol" | null = null;
  let i = 0;

  const flushPara = () => {
    if (para.length) {
      html.push(`<p>${para.map(inline).join("<br>")}</p>`);
      para = [];
    }
  };
  const closeList = () => {
    if (listType) {
      html.push(`</${listType}>`);
      listType = null;
    }
  };

  while (i < lines.length) {
    const line = lines[i];

    // 围栏代码块
    if (line.trim().startsWith("```")) {
      flushPara();
      closeList();
      const buf: string[] = [];
      i++;
      while (i < lines.length && !lines[i].trim().startsWith("```")) {
        buf.push(lines[i]);
        i++;
      }
      i++; // 跳过结束围栏
      html.push(`<pre><code>${escapeHtml(buf.join("\n"))}</code></pre>`);
      continue;
    }
    // 空行
    if (line.trim() === "") {
      flushPara();
      closeList();
      i++;
      continue;
    }
    // 标题
    const h = line.match(/^(#{1,4})\s+(.*)$/);
    if (h) {
      flushPara();
      closeList();
      html.push(`<h${h[1].length}>${inline(h[2])}</h${h[1].length}>`);
      i++;
      continue;
    }
    // 分隔线
    if (/^\s*-{3,}\s*$/.test(line)) {
      flushPara();
      closeList();
      html.push("<hr>");
      i++;
      continue;
    }
    // 引用
    if (/^\s*>\s?/.test(line)) {
      flushPara();
      closeList();
      html.push(`<blockquote>${inline(line.replace(/^\s*>\s?/, ""))}</blockquote>`);
      i++;
      continue;
    }
    // 列表
    const ul = line.match(/^\s*[-*]\s+(.*)$/);
    const ol = line.match(/^\s*\d+\.\s+(.*)$/);
    if (ul || ol) {
      flushPara();
      const type: "ul" | "ol" = ul ? "ul" : "ol";
      if (listType !== type) {
        closeList();
        html.push(`<${type}>`);
        listType = type;
      }
      html.push(`<li>${inline((ul ?? ol)![1])}</li>`);
      i++;
      continue;
    }
    // 普通段落行（段落内换行保留为 <br>）
    para.push(line);
    i++;
  }
  flushPara();
  closeList();
  return html.join("\n");
}
