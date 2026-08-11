export type Diagnostic = {
  step: string;
  tool: string;
  severity: string;
  message: string;
  path?: string;
  range?: {
    start: { line: number; column?: number };
    end?: { line: number; column?: number };
  };
  rule?: string;
  help_url?: string;
  fix?: { replacement?: string };
};

export type StepResult = {
  name: string;
  status: string;
  started_at?: string;
  duration_ms: number;
  diagnostics?: Diagnostic[];
  effects?: Array<{ command: string; effect?: string }>;
  skip_reason?: string;
};

export type RunSnapshot = {
  id: string;
  root: string;
  kind: string;
  status: string;
  started_at: string;
  finished_at?: string;
  output_bytes: number;
  output_truncated?: boolean;
  has_diff: boolean;
  diff_bytes?: number;
  diff_truncated?: boolean;
  result?: {
    status: string;
    duration_ms?: number;
    steps: StepResult[];
    failure?: string;
  };
  error?: string;
};

export type TextPage = {
  text?: string;
  offset?: number;
  next_offset?: number;
  total_bytes?: number;
  eof?: boolean;
  truncated?: boolean;
};
