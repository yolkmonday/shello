import type { Terminal } from "@xterm/xterm";
import type { ITheme } from "@xterm/xterm";

const ANSI_COLORS_16 = [
  "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
  "brightBlack", "brightRed", "brightGreen", "brightYellow", "brightBlue", "brightMagenta", "brightCyan", "brightWhite",
] as const;

function color256ToHex(c: number): string {
  if (c < 16) return ""; // handled by theme
  if (c < 232) {
    const idx = c - 16;
    const r = Math.floor(idx / 36) * 51;
    const g = Math.floor((idx % 36) / 6) * 51;
    const b = (idx % 6) * 51;
    return `rgb(${r},${g},${b})`;
  }
  const gray = (c - 232) * 10 + 8;
  return `rgb(${gray},${gray},${gray})`;
}

function resolveColor(colorNumber: number, colorMode: number, theme: ITheme, isPalette16: boolean): string | null {
  if (colorMode === 0) return null; // default
  if (colorMode === 1 || isPalette16) {
    // 16-color palette
    const name = ANSI_COLORS_16[colorNumber];
    if (name && theme[name]) return theme[name] as string;
    return null;
  }
  if (colorMode === 2) {
    // 256-color
    if (colorNumber < 16) {
      const name = ANSI_COLORS_16[colorNumber];
      if (name && theme[name]) return theme[name] as string;
    }
    return color256ToHex(colorNumber);
  }
  if (colorMode === 3) {
    // 24-bit RGB packed as single number
    const r = (colorNumber >> 16) & 0xff;
    const g = (colorNumber >> 8) & 0xff;
    const b = colorNumber & 0xff;
    return `rgb(${r},${g},${b})`;
  }
  return null;
}

export interface SerializedTerminal {
  html: string;
  rows: number;
  cols: number;
  foreground: string;
  background: string;
  fontFamily: string;
  fontSize: number;
}

export function serializeTerminalBuffer(terminal: Terminal, theme: ITheme): SerializedTerminal {
  const buffer = terminal.buffer.active;
  const cols = terminal.cols;
  const lines: string[] = [];

  // Find the last non-empty line to avoid trailing blank lines
  let lastNonEmpty = -1;
  for (let y = 0; y < buffer.length; y++) {
    const line = buffer.getLine(y);
    if (!line) continue;
    const text = line.translateToString(false);
    if (text.trim().length > 0) lastNonEmpty = y;
  }

  // Only render visible rows from viewport if buffer is short
  const startY = Math.max(0, lastNonEmpty - terminal.rows + 1);
  const endY = lastNonEmpty + 1;

  for (let y = startY; y < endY; y++) {
    const line = buffer.getLine(y);
    if (!line) { lines.push(""); continue; }

    let lineHtml = "";
    let x = 0;
    while (x < cols) {
      const cell = line.getCell(x);
      if (!cell) { x++; continue; }

      const char = cell.getChars() || " ";
      const width = cell.getWidth();
      if (width === 0) { x++; continue; }

      const fgMode = cell.getFgColorMode();
      const bgMode = cell.getBgColorMode();
      const fgColor = resolveColor(cell.getFgColor(), fgMode, theme, fgMode === 1);
      const bgColor = resolveColor(cell.getBgColor(), bgMode, theme, bgMode === 1);
      const bold = cell.isBold();
      const italic = cell.isItalic();
      const underline = cell.isUnderline();
      const dim = cell.isDim();

      const styles: string[] = [];
      if (fgColor) styles.push(`color:${fgColor}`);
      if (bgColor) styles.push(`background:${bgColor}`);
      if (bold) styles.push("font-weight:bold");
      if (italic) styles.push("font-style:italic");
      if (underline) styles.push("text-decoration:underline");
      if (dim) styles.push("opacity:0.5");

      const escaped = char
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;");

      if (styles.length > 0) {
        lineHtml += `<span style="${styles.join(";")}">${escaped}</span>`;
      } else {
        lineHtml += escaped;
      }

      x += width;
    }

    lines.push(lineHtml);
  }

  // Trim trailing empty lines from rendered output
  while (lines.length > 0 && lines[lines.length - 1].trim() === "") {
    lines.pop();
  }

  return {
    html: lines.join("\n"),
    rows: lines.length,
    cols,
    foreground: (theme.foreground as string) || "#EDEDF0",
    background: (theme.background as string) || "#121214",
    fontFamily: `'${terminal.options.fontFamily || "monospace"}'`,
    fontSize: terminal.options.fontSize || 14,
  };
}
