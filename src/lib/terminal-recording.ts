import type { ThemeName } from "./themes";

export interface RecordingEvent {
  /** ms since recording started (excluding paused time) */
  time: number;
  /** terminal data (text) */
  data: string;
}

export interface TerminalRecording {
  events: RecordingEvent[];
  duration: number;
  cols: number;
  rows: number;
  theme: ThemeName;
  fontSize: number;
  fontFamily: string;
}

export class TerminalRecorder {
  private events: RecordingEvent[] = [];
  private startTime = 0;
  private pausedAt = 0;
  private pausedTotal = 0;
  private _recording = false;
  private _paused = false;

  get recording() { return this._recording; }
  get paused() { return this._paused; }

  /** Elapsed recording time in ms (excluding paused time) */
  get elapsed(): number {
    if (!this._recording) return 0;
    if (this._paused) return this.pausedAt - this.startTime - this.pausedTotal;
    return performance.now() - this.startTime - this.pausedTotal;
  }

  start() {
    this.events = [];
    this.startTime = performance.now();
    this.pausedAt = 0;
    this.pausedTotal = 0;
    this._recording = true;
    this._paused = false;
  }

  pause() {
    if (!this._recording || this._paused) return;
    this._paused = true;
    this.pausedAt = performance.now();
  }

  resume() {
    if (!this._recording || !this._paused) return;
    this.pausedTotal += performance.now() - this.pausedAt;
    this._paused = false;
  }

  write(data: string) {
    if (!this._recording || this._paused) return;
    this.events.push({
      time: performance.now() - this.startTime - this.pausedTotal,
      data,
    });
  }

  stop(cols: number, rows: number, theme: ThemeName, fontSize: number, fontFamily: string): TerminalRecording {
    if (this._paused) this.resume();
    this._recording = false;
    const duration = this.events.length > 0
      ? this.events[this.events.length - 1].time
      : 0;
    return {
      events: [...this.events],
      duration,
      cols,
      rows,
      theme,
      fontSize,
      fontFamily,
    };
  }
}
