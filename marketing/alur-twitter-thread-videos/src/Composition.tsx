import type { CSSProperties, ReactNode } from "react";
import { AbsoluteFill, Easing, interpolate, Sequence, spring, useCurrentFrame, useVideoConfig } from "remotion";

type Accent = keyof typeof colors;
type Visual = "confusion" | "featureLaunch" | "fastDelegate" | "nodePassthrough";
type CodeLanguage = "json" | "shell" | "text";

type VideoSpec = {
  id: string;
  part: string;
  kicker: string;
  headline: string;
  subhead: string;
  footer: string;
  durationInFrames: number;
  visual: Visual;
  thumbnail: {
    title: string;
    subtitle: string;
    chips: string[];
    accent: Accent;
  };
};

export type ThreadVideoProps = {
  spec: VideoSpec;
};

const colors = {
  ink: "#111111",
  panel: "#151515",
  paper: "#f8f1e5",
  muted: "#a9a39a",
  line: "#34312c",
  green: "#33d17a",
  cyan: "#55c7ff",
  yellow: "#f6c85f",
  violet: "#9d7cff",
  red: "#ff6b6b",
};

const mono: CSSProperties = {
  fontFamily: "\"SFMono-Regular\", \"Cascadia Code\", \"Liberation Mono\", Menlo, Consolas, monospace",
};

type CodeToken = {
  text: string;
  color: string;
};

const full: CSSProperties = {
  fontFamily:
    "\"SF Pro Display\", \"Inter\", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, \"Segoe UI\", sans-serif",
  color: colors.paper,
  backgroundColor: colors.ink,
};

const ease = Easing.bezier(0.16, 1, 0.3, 1);
const clamp = {
  extrapolateLeft: "clamp",
  extrapolateRight: "clamp",
  easing: ease,
} as const;

const appear = (frame: number, start: number, duration = 26) =>
  interpolate(frame, [start, start + duration], [0, 1], clamp);

const rise = (frame: number, start: number, amount = 24) => interpolate(frame, [start, start + 26], [amount, 0], clamp);

const stringTokens = (text: string): CodeToken[] => {
  const tokens: CodeToken[] = [];
  const stringPattern = /"[^"]*"/g;
  let lastIndex = 0;
  let match: RegExpExecArray | null;

  while ((match = stringPattern.exec(text)) !== null) {
    if (match.index > lastIndex) {
      tokens.push({
        text: text.slice(lastIndex, match.index),
        color: colors.muted,
      });
    }
    tokens.push({ text: match[0], color: colors.yellow });
    lastIndex = match.index + match[0].length;
  }

  if (lastIndex < text.length) {
    tokens.push({ text: text.slice(lastIndex), color: colors.muted });
  }

  return tokens;
};

const codeTokens = (line: string, language: CodeLanguage): CodeToken[] => {
  if (language === "json") {
    const key = line.match(/^(\s*)("[^"]+")(:)(.*)$/);
    if (key) {
      return [
        { text: key[1], color: colors.muted },
        { text: key[2], color: colors.cyan },
        { text: key[3], color: colors.muted },
        ...stringTokens(key[4]),
      ];
    }

    return stringTokens(line).length > 0
      ? stringTokens(line)
      : [{ text: line, color: colors.muted }];
  }

  if (language === "shell") {
    const command = line.match(/^(\s*)(\$?\s?)([^\s|]+)(.*)$/);
    if (command) {
      return [
        { text: command[1], color: colors.muted },
        { text: command[2], color: colors.green },
        { text: command[3], color: colors.cyan },
        { text: command[4], color: colors.paper },
      ];
    }
  }

  return [{ text: line, color: colors.paper }];
};

export const videos: VideoSpec[] = [
  {
    id: "alur-launch-node-commands",
    part: "01/04",
    kicker: "one command everywhere",
    headline: "One vocabulary. Two ways in.",
    subhead:
      "Use ni, nr, and nex across package managers. Turn on the optional Node shim when you want node install too.",
    footer: "same commands across npm, pnpm, yarn, bun, and deno",
    durationInFrames: 480,
    visual: "confusion",
    thumbnail: {
      title: "Introducing alur",
      subtitle: "Package-manager commands + optional node install",
      chips: ["ni vite", "nr dev", "node install", "node run"],
      accent: "green",
    },
  },
  {
    id: "alur-launch-detects-pm",
    part: "02/04",
    kicker: "same commands everywhere",
    headline: "One CLI. Every package manager.",
    subhead:
      "Switch projects without switching muscle memory: install, run, exec, clean install, uninstall, parallel, sequential.",
    footer: "ni | nr | nex | nci | nrm | npar | nseq",
    durationInFrames: 1110,
    visual: "featureLaunch",
    thumbnail: {
      title: "Every package manager. One CLI.",
      subtitle: "Install, run, exec, clean install, parallel, sequential",
      chips: ["ni", "nr", "nex", "nci", "nrm", "npar", "nseq"],
      accent: "yellow",
    },
  },
  {
    id: "alur-launch-fast-delegates",
    part: "03/04",
    kicker: "speed without surprises",
    headline: "Fast when it can.",
    subhead:
      "alur runs local scripts and tools faster when it can. When compatibility matters, it falls back automatically.",
    footer: "faster when safe | compatible when needed",
    durationInFrames: 540,
    visual: "fastDelegate",
    thumbnail: {
      title: "Introducing fast mode",
      subtitle: "Faster when safe. Compatible when needed.",
      chips: ["nr --fast", "nex", "node run"],
      accent: "green",
    },
  },
  {
    id: "alur-launch-normal-node",
    part: "04/04",
    kicker: "node stays node",
    headline: "Use node install. Keep Node intact.",
    subhead: "Package commands become easier. Regular Node commands keep working exactly like before.",
    footer: "alur.happytoolin.com",
    durationInFrames: 420,
    visual: "nodePassthrough",
    thumbnail: {
      title: "Optional Node shim",
      subtitle: "Package commands for Node projects",
      chips: ["node install", "node run", "node exec"],
      accent: "cyan",
    },
  },
];

export const ThreadVideo = ({ spec }: ThreadVideoProps) => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const bgDrift = interpolate(frame, [0, durationInFrames], [-18, 18], clamp);

  if (spec.visual === "featureLaunch") {
    return <FeatureLaunchVideo spec={spec} />;
  }

  return (
    <AbsoluteFill style={full}>
      <div
        style={{
          position: "absolute",
          inset: 0,
          background: "linear-gradient(135deg, #111111 0%, #17130f 48%, #101419 100%)",
        }}
      />
      <div
        style={{
          position: "absolute",
          right: -50 + bgDrift,
          bottom: -24,
          fontSize: 156,
          fontWeight: 900,
          color: "rgba(248,241,229,0.035)",
          letterSpacing: 0,
          ...mono,
        }}
      >
        alur
      </div>
      <Header spec={spec} />
      <Message spec={spec} />
      <VisualPanel>
        {spec.visual === "confusion" ? <ConfusionVisual /> : null}
        {spec.visual === "fastDelegate" ? <FastDelegateVisual /> : null}
        {spec.visual === "nodePassthrough" ? <NodePassthroughVisual /> : null}
      </VisualPanel>
      <Footer label={spec.footer} />
      <ThumbnailIntro spec={spec} />
    </AbsoluteFill>
  );
};

const ThumbnailIntro = ({ spec }: { spec: VideoSpec }) => {
  const frame = useCurrentFrame();

  if (frame > 54) {
    return null;
  }

  const opacity = interpolate(frame, [30, 54], [1, 0], clamp);
  const scale = interpolate(frame, [0, 54], [1, 1.025], clamp);
  const accent = colors[spec.thumbnail.accent];

  return (
    <AbsoluteFill
      style={{
        opacity,
        background:
          "radial-gradient(circle at 18% 18%, rgba(51,209,122,0.18), transparent 28%), radial-gradient(circle at 82% 76%, rgba(246,200,95,0.14), transparent 30%), #0c0a09",
        color: colors.paper,
        overflow: "hidden",
      }}
    >
      <div
        style={{
          position: "absolute",
          inset: 0,
          opacity: 0.22,
          backgroundImage:
            "linear-gradient(rgba(248,241,229,0.08) 1px, transparent 1px), linear-gradient(90deg, rgba(248,241,229,0.08) 1px, transparent 1px)",
          backgroundSize: "48px 48px",
        }}
      />
      <div
        style={{
          position: "absolute",
          right: 52,
          bottom: -36,
          color: "rgba(248,241,229,0.04)",
          fontSize: 190,
          fontWeight: 900,
          ...mono,
        }}
      >
        alur
      </div>
      <div
        style={{
          position: "absolute",
          top: 42,
          left: 56,
          right: 56,
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          color: colors.muted,
          fontSize: 18,
          textTransform: "uppercase",
        }}
      >
        <span>alur for JavaScript projects</span>
        <span style={mono}>{spec.part}</span>
      </div>
      <div
        style={{
          position: "absolute",
          left: 76,
          right: 76,
          top: 150,
          transform: `scale(${scale})`,
          transformOrigin: "left center",
        }}
      >
        <div
          style={{
            color: accent,
            fontSize: 20,
            textTransform: "uppercase",
            marginBottom: 18,
            ...mono,
          }}
        >
          alur
        </div>
        <div
          style={{
            fontSize: spec.thumbnail.title.length > 34 ? 58 : 68,
            lineHeight: 0.94,
            fontWeight: 900,
            maxWidth: 880,
          }}
        >
          {spec.thumbnail.title}
        </div>
        <div
          style={{
            color: colors.muted,
            fontSize: 28,
            lineHeight: 1.18,
            marginTop: 24,
            maxWidth: 760,
          }}
        >
          {spec.thumbnail.subtitle}
        </div>
        <div style={{ display: "flex", gap: 12, marginTop: 34, flexWrap: "wrap" }}>
          {spec.thumbnail.chips.map((chip) => (
            <span
              key={chip}
              style={{
                border: `1px solid ${accent}`,
                color: chip === "pnpm@11" ? colors.yellow : colors.paper,
                backgroundColor: "#111111",
                padding: "10px 15px",
                fontSize: 18,
                ...mono,
              }}
            >
              {chip}
            </span>
          ))}
        </div>
      </div>
    </AbsoluteFill>
  );
};

const Header = ({ spec }: { spec: VideoSpec }) => (
  <div
    style={{
      position: "absolute",
      top: 34,
      left: 56,
      right: 56,
      display: "flex",
      alignItems: "center",
      justifyContent: "space-between",
      fontSize: 18,
      color: colors.muted,
      textTransform: "uppercase",
      letterSpacing: 0,
    }}
  >
    <span>{spec.kicker}</span>
    <span style={mono}>{spec.part}</span>
  </div>
);

const Message = ({ spec }: { spec: VideoSpec }) => {
  const frame = useCurrentFrame();

  return (
    <>
      <div
        style={{
          position: "absolute",
          top: 86,
          left: 56,
          width: 520,
          opacity: appear(frame, 0, 34),
          transform: `translateY(${rise(frame, 0, 34)}px)`,
        }}
      >
        <div
          style={{
            fontSize: spec.headline.length > 28 ? 50 : 58,
            lineHeight: 0.96,
            fontWeight: 900,
            letterSpacing: 0,
          }}
        >
          {spec.headline}
        </div>
      </div>
      <div
        style={{
          position: "absolute",
          top: 260,
          left: 58,
          width: 508,
          fontSize: 24,
          lineHeight: 1.22,
          color: colors.muted,
          opacity: appear(frame, 36, 34),
          transform: `translateY(${rise(frame, 36, 20)}px)`,
        }}
      >
        {spec.subhead}
      </div>
    </>
  );
};

const VisualPanel = ({ children }: { children: ReactNode }) => (
  <div
    style={{
      position: "absolute",
      left: 604,
      right: 44,
      top: 92,
      height: 512,
      padding: 28,
      border: `2px solid ${colors.line}`,
      backgroundColor: colors.panel,
      boxShadow: "0 28px 0 rgba(0,0,0,0.24)",
    }}
  >
    {children}
  </div>
);

const Footer = ({ label }: { label: string }) => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const progress = interpolate(frame, [0, durationInFrames - 1], [0, 1], clamp);

  return (
    <div
      style={{
        position: "absolute",
        left: 56,
        right: 56,
        bottom: 28,
      }}
    >
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          color: colors.muted,
          fontSize: 18,
        }}
      >
        <span>{label}</span>
        <span style={{ color: colors.paper, ...mono }}>@happytoolin</span>
      </div>
      <div
        style={{
          height: 3,
          marginTop: 14,
          backgroundColor: "rgba(248,241,229,0.12)",
        }}
      >
        <div
          style={{
            width: `${progress * 100}%`,
            height: "100%",
            backgroundColor: colors.green,
          }}
        />
      </div>
    </div>
  );
};

const ConfusionVisual = () => {
  const frame = useCurrentFrame();
  const sceneFrame = Math.max(0, frame - 54);
  const projects = [
    {
      name: "web-app",
      pm: "npm",
      command: "npm i vite",
      accent: "red" as Accent,
    },
    {
      name: "dashboard",
      pm: "pnpm",
      command: "pnpm add vite",
      accent: "green" as Accent,
    },
    {
      name: "docs",
      pm: "yarn",
      command: "yarn add vite",
      accent: "cyan" as Accent,
    },
    {
      name: "edge-api",
      pm: "bun",
      command: "bun add vite",
      accent: "violet" as Accent,
    },
    {
      name: "automation",
      pm: "deno",
      command: "deno add vite",
      accent: "yellow" as Accent,
    },
  ];
  const active = Math.min(projects.length - 1, Math.floor(sceneFrame / 52));
  const commandOpacity = appear(sceneFrame, 258, 26);
  const shimOpacity = appear(sceneFrame, 318, 26);

  return (
    <>
      <div style={{ fontSize: 18, color: colors.muted, textTransform: "uppercase" }}>
        different projects, different defaults
      </div>
      <div style={{ marginTop: 14, display: "grid", gap: 6 }}>
        {projects.map((project, index) => (
          <ProjectRow
            key={project.name}
            active={active === index}
            index={index}
            project={project}
            frame={sceneFrame}
          />
        ))}
      </div>
      <div
        style={{
          position: "absolute",
          left: 28,
          right: 28,
          bottom: 28,
          display: "grid",
          gridTemplateColumns: "1fr 1fr",
          gap: 12,
        }}
      >
        <LaunchHook
          label="one command everywhere"
          command="ni vite"
          note="npm / pnpm@11 / yarn / bun / deno"
          accent="green"
          opacity={commandOpacity}
          y={rise(sceneFrame, 258, 16)}
        />
        <LaunchHook
          label="optional Node shim"
          command="node install vite"
          note="package commands, Node-style"
          accent="cyan"
          opacity={shimOpacity}
          y={rise(sceneFrame, 318, 16)}
        />
      </div>
    </>
  );
};

const LaunchHook = ({
  label,
  command,
  note,
  accent,
  opacity,
  y,
}: {
  label: string;
  command: string;
  note: string;
  accent: Accent;
  opacity: number;
  y: number;
}) => (
  <div
    style={{
      opacity,
      transform: `translateY(${y}px)`,
      border: `1px solid ${colors[accent]}`,
      backgroundColor: "#111111",
      padding: "13px 14px",
      minWidth: 0,
      boxSizing: "border-box",
    }}
  >
    <div
      style={{
        color: colors[accent],
        fontSize: 12,
        textTransform: "uppercase",
        marginBottom: 8,
        ...mono,
      }}
    >
      {label}
    </div>
    <div style={{ color: colors.paper, fontSize: 20, whiteSpace: "nowrap", ...mono }}>
      $ {command}
    </div>
    <div style={{ color: colors.muted, fontSize: 11, marginTop: 8, lineHeight: 1.25 }}>
      {note}
    </div>
  </div>
);

const ProjectRow = ({
  project,
  active,
  index,
  frame,
}: {
  project: {
    name: string;
    pm: string;
    command: string;
    accent: Accent;
  };
  active: boolean;
  index: number;
  frame: number;
}) => {
  const opacity = appear(frame, index * 22, 22);
  const accent = colors[project.accent];

  return (
    <div
      style={{
        opacity,
        transform: `translateY(${rise(frame, index * 22, 14)}px) scale(${active ? 1.025 : 1})`,
        display: "grid",
        gridTemplateColumns: "96px 64px 1fr",
        gap: 8,
        alignItems: "center",
        padding: "7px 10px",
        border: `2px solid ${active ? accent : colors.line}`,
        backgroundColor: active ? "rgba(248,241,229,0.055)" : "#111111",
      }}
    >
      <span style={{ fontSize: 14, color: colors.paper }}>{project.name}</span>
      <span style={{ fontSize: 12, color: accent, ...mono }}>{project.pm}</span>
      <span style={{ fontSize: 12, color: colors.muted, ...mono }}>{project.command}</span>
    </div>
  );
};

const FeatureLaunchVideo = ({ spec }: { spec: VideoSpec }) => {
  const frame = useCurrentFrame();
  const durations = [135, 165, 210, 150, 150, 135, 165];
  const starts = durations.reduce<number[]>((acc, duration, index) => {
    acc.push(index === 0 ? 0 : acc[index - 1] + durations[index - 1]);
    return acc;
  }, []);
  const bg = interpolate(frame, [0, spec.durationInFrames], [0, 1], clamp);

  return (
    <AbsoluteFill style={full}>
      <div
        style={{
          position: "absolute",
          inset: 0,
          background:
            "radial-gradient(circle at 18% 16%, rgba(51,209,122,0.16), transparent 28%), radial-gradient(circle at 82% 78%, rgba(246,200,95,0.12), transparent 30%), #0c0a09",
        }}
      />
      <div
        style={{
          position: "absolute",
          inset: 0,
          opacity: 0.22,
          backgroundImage:
            "linear-gradient(rgba(248,241,229,0.08) 1px, transparent 1px), linear-gradient(90deg, rgba(248,241,229,0.08) 1px, transparent 1px)",
          backgroundSize: "44px 44px",
          transform: `translateY(${bg * -24}px)`,
        }}
      />
      <div
        style={{
          position: "absolute",
          top: 28,
          left: 44,
          right: 44,
          display: "flex",
          justifyContent: "space-between",
          color: colors.muted,
          fontSize: 18,
          textTransform: "uppercase",
          letterSpacing: 0,
        }}
      >
        <span>{spec.kicker}</span>
        <span style={mono}>{spec.part}</span>
      </div>
      <Sequence from={starts[0]} durationInFrames={durations[0]}>
        <LaunchScene duration={durations[0]}>
          <TerminalInstallScene />
        </LaunchScene>
      </Sequence>
      <Sequence from={starts[1]} durationInFrames={durations[1]}>
        <LaunchScene duration={durations[1]}>
          <PackageSwitchScene />
        </LaunchScene>
      </Sequence>
      <Sequence from={starts[2]} durationInFrames={durations[2]}>
        <LaunchScene duration={durations[2]}>
          <CommandCatalogScene />
        </LaunchScene>
      </Sequence>
      <Sequence from={starts[3]} durationInFrames={durations[3]}>
        <LaunchScene duration={durations[3]}>
          <ParallelSequentialScene />
        </LaunchScene>
      </Sequence>
      <Sequence from={starts[4]} durationInFrames={durations[4]}>
        <LaunchScene duration={durations[4]}>
          <FastDelegateLaunchScene />
        </LaunchScene>
      </Sequence>
      <Sequence from={starts[5]} durationInFrames={durations[5]}>
        <LaunchScene duration={durations[5]}>
          <NodeShimLaunchScene />
        </LaunchScene>
      </Sequence>
      <Sequence from={starts[6]} durationInFrames={durations[6]}>
        <LaunchScene duration={durations[6]}>
          <LaunchCtaScene />
        </LaunchScene>
      </Sequence>
      <Footer label={spec.footer} />
      <ThumbnailIntro spec={spec} />
    </AbsoluteFill>
  );
};

const LaunchScene = ({
  children,
  duration,
}: {
  children: ReactNode;
  duration: number;
}) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const enter = spring({
    frame,
    fps,
    durationInFrames: 28,
    config: { damping: 18, stiffness: 130 },
  });
  const exit = interpolate(frame, [duration - 24, duration], [1, 0], clamp);
  const progress = enter * exit;

  return (
    <div
      style={{
        position: "absolute",
        inset: "76px 58px 82px",
        opacity: progress,
        transform: `scale(${interpolate(progress, [0, 1], [0.95, 1])}) translateY(${
          interpolate(progress, [0, 1], [26, 0])
        }px)`,
      }}
    >
      {children}
    </div>
  );
};

const AppWindow = ({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) => {
  return (
    <div
      style={{
        height: "100%",
        border: `1px solid ${colors.line}`,
        backgroundColor: "#151515",
        boxShadow: "0 30px 90px rgba(0,0,0,0.42)",
        overflow: "hidden",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          height: 42,
          padding: "0 16px",
          borderBottom: `1px solid ${colors.line}`,
          backgroundColor: "#1c1917",
          color: colors.muted,
          fontSize: 13,
          ...mono,
        }}
      >
        <div style={{ display: "flex", gap: 7, marginRight: 14 }}>
          {["#ff5f57", "#ffbd2e", "#28c840"].map((dot) => (
            <span
              key={dot}
              style={{
                width: 11,
                height: 11,
                borderRadius: 999,
                backgroundColor: dot,
                display: "block",
              }}
            />
          ))}
        </div>
        <span>{title}</span>
      </div>
      <div style={{ height: "calc(100% - 42px)", padding: 28, boxSizing: "border-box" }}>
        {children}
      </div>
    </div>
  );
};

const TerminalInstallScene = () => {
  const frame = useCurrentFrame();
  const command = "npm i -g @happytoolin/alur";
  const typed = command.slice(0, Math.min(command.length, Math.floor(frame / 1.25)));
  const output = [
    "ready for every repo",
    "commands: ni nr nex nci nrm npar nseq",
    "works with npm, pnpm@11, yarn, bun, deno",
    "optional: node install, node run, node exec",
  ];

  return (
    <AppWindow title="install alur">
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "1fr 310px",
          gap: 28,
          height: "100%",
          alignItems: "center",
        }}
      >
        <div style={{ ...mono, fontSize: 22, lineHeight: 1.45 }}>
          <div style={{ color: colors.green }}>
            $ {typed}
            <span style={{ color: colors.paper }}>_</span>
          </div>
          <div style={{ marginTop: 26, color: colors.yellow, fontSize: 34, fontWeight: 800 }}>
            ALUR
          </div>
          <div style={{ marginTop: 10, display: "grid", gap: 8 }}>
            {output.map((line, index) => (
              <div
                key={line}
                style={{
                  opacity: appear(frame, command.length + 16 + index * 12, 12),
                  color: index === 1 ? colors.green : colors.muted,
                }}
              >
                {line}
              </div>
            ))}
          </div>
        </div>
        <LaunchBadge
          title="Stop switching tools"
          lines={["type one command", "keep each project's choice", "use Node-style package commands"]}
        />
      </div>
    </AppWindow>
  );
};

const PackageSwitchScene = () => {
  const frame = useCurrentFrame();
  const projects = [
    ["web-app", "npm", "package-lock.json", "npm i vite", "red"],
    ["dashboard", "pnpm", "pnpm-lock.yaml", "pnpm add vite", "green"],
    ["docs", "yarn", "yarn.lock", "yarn add vite", "cyan"],
    ["edge-api", "bun", "bun.lock", "bun add vite", "violet"],
    ["automation", "deno", "deno.lock", "deno add vite", "yellow"],
  ] as const;
  const active = Math.min(projects.length - 1, Math.floor(frame / 30));
  const selected = projects[active];

  return (
    <AppWindow title="move between projects">
      <div style={{ display: "grid", gridTemplateColumns: "330px 1fr", gap: 28, height: "100%" }}>
        <div style={{ display: "grid", gap: 10, alignContent: "center" }}>
          {projects.map(([name, pm, , , accent], index) => (
            <div
              key={name}
              style={{
                opacity: appear(frame, index * 10, 12),
                display: "grid",
                gridTemplateColumns: "94px 54px 1fr",
                gap: 10,
                alignItems: "center",
                border: `2px solid ${index === active ? colors[accent] : colors.line}`,
                backgroundColor: index === active ? "rgba(248,241,229,0.06)" : "#111111",
                padding: "12px 14px",
                ...mono,
              }}
            >
              <span style={{ color: colors.paper, fontSize: 16 }}>{name}</span>
              <span style={{ color: colors[accent], fontSize: 14 }}>{pm}</span>
              <span style={{ color: colors.muted, fontSize: 12 }}>project</span>
            </div>
          ))}
        </div>
        <div
          style={{
            display: "grid",
            gridTemplateRows: "1fr 58px 1fr",
            alignItems: "center",
            height: "100%",
          }}
        >
          <LaunchPanel title="you type" accent="green">
            <div style={{ color: colors.green, fontSize: 34, ...mono }}>$ ni vite</div>
          </LaunchPanel>
          <div style={{ color: colors.muted, textAlign: "center", fontSize: 18 }}>
            alur keeps the repo's package manager
          </div>
          <LaunchPanel title={`${selected[1]} project`} accent={selected[4]}>
            <div style={{ color: colors[selected[4]], fontSize: 30, ...mono }}>$ {selected[3]}</div>
          </LaunchPanel>
        </div>
      </div>
    </AppWindow>
  );
};

const CommandCatalogScene = () => {
  const frame = useCurrentFrame();
  const commands = [
    ["ni", "install packages", "ni vite", "add dependencies", "green"],
    ["nr", "run scripts", "nr dev", "start dev", "cyan"],
    ["nex", "run tools", "nex vitest", "project CLIs", "violet"],
    ["nci", "clean install", "nci", "fresh install", "yellow"],
    ["nrm", "remove packages", "nrm lodash", "remove dependency", "red"],
    ["npar", "parallel tasks", "npar \"lint\" \"test\"", "together", "green"],
    ["nseq", "ordered tasks", "nseq \"clean\" \"build\"", "in order", "cyan"],
    ["node", "optional shim", "node install vite", "Node-style commands", "yellow"],
  ] as const;

  return (
    <AppWindow title="commands you can remember">
      <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 14, height: "100%" }}>
        {commands.map(([cmd, label, typed, result, accent], index) => (
          <div
            key={cmd}
            style={{
              opacity: appear(frame, index * 9, 14),
              transform: `translateY(${rise(frame, index * 9, 12)}px)`,
              border: `1px solid ${colors[accent]}`,
              backgroundColor: "#111111",
              padding: 16,
              minHeight: 142,
              boxSizing: "border-box",
            }}
          >
            <div style={{ color: colors[accent], fontSize: 28, fontWeight: 900, ...mono }}>{cmd}</div>
            <div style={{ color: colors.paper, fontSize: 17, marginTop: 8 }}>{label}</div>
            <div style={{ color: colors.muted, fontSize: 13, marginTop: 16, ...mono }}>$ {typed}</div>
            <div style={{ color: colors[accent], fontSize: 12, marginTop: 8, ...mono }}>{result}</div>
          </div>
        ))}
      </div>
    </AppWindow>
  );
};

const ParallelSequentialScene = () => {
  const frame = useCurrentFrame();
  const parallel = ["lint", "test", "build"];
  const sequential = ["clean", "build", "deploy"];

  return (
    <AppWindow title="run tasks your way">
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 26, height: "100%" }}>
        <LaunchPanel title='npar "lint" "test" "build"' accent="green">
          <div style={{ display: "grid", gap: 18, marginTop: 22 }}>
            {parallel.map((task, index) => (
              <TaskBar key={task} label={task} progressFrame={frame - index * 6} accent="green" />
            ))}
          </div>
          <div style={{ color: colors.muted, marginTop: 26, fontSize: 16 }}>Parallel jobs start together.</div>
        </LaunchPanel>
        <LaunchPanel title='nseq "clean" "build" "deploy"' accent="yellow">
          <div style={{ display: "grid", gap: 18, marginTop: 22 }}>
            {sequential.map((task, index) => (
              <TaskBar key={task} label={task} progressFrame={frame - index * 38} accent="yellow" />
            ))}
          </div>
          <div style={{ color: colors.muted, marginTop: 26, fontSize: 16 }}>Sequential jobs wait their turn.</div>
        </LaunchPanel>
      </div>
    </AppWindow>
  );
};

const FastDelegateLaunchScene = () => {
  const frame = useCurrentFrame();

  return (
    <AppWindow title="fast, with fallback">
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 26, height: "100%", alignItems: "center" }}>
        <LaunchPanel title="faster when safe" accent="green">
          <FeatureStep label="local scripts" from={0} frame={frame} accent="green" />
          <FeatureStep label="project tools" from={18} frame={frame} accent="green" />
          <FeatureStep label="less startup time" from={36} frame={frame} accent="green" />
        </LaunchPanel>
        <LaunchPanel title="compatible when needed" accent="yellow">
          <FeatureStep label="complex workspaces" from={54} frame={frame} accent="yellow" />
          <FeatureStep label="remote packages" from={72} frame={frame} accent="yellow" />
          <FeatureStep label="automatic fallback" from={90} frame={frame} accent="yellow" />
        </LaunchPanel>
      </div>
    </AppWindow>
  );
};

const NodeShimLaunchScene = () => {
  const frame = useCurrentFrame();
  const rows = [
    ["node -v", "unchanged", "cyan"],
    ["node script.js", "unchanged", "cyan"],
    ["node install vite", "package command", "green"],
    ["node run dev", "package command", "green"],
    ["node exec vitest", "package command", "green"],
  ] as const;

  return (
    <AppWindow title="node commands, upgraded">
      <div style={{ display: "grid", gap: 12, marginTop: 18 }}>
        {rows.map(([command, target, accent], index) => (
          <div
            key={command}
            style={{
              opacity: appear(frame, index * 14, 14),
              display: "grid",
              gridTemplateColumns: "1fr 150px",
              alignItems: "center",
              padding: "16px 18px",
              border: `1px solid ${colors[accent]}`,
              backgroundColor: "#111111",
              ...mono,
            }}
          >
            <span style={{ color: colors.paper, fontSize: 24 }}>$ {command}</span>
            <span style={{ color: colors[accent], textAlign: "right", fontSize: 17 }}>{target}</span>
          </div>
        ))}
      </div>
    </AppWindow>
  );
};

const LaunchCtaScene = () => {
  const frame = useCurrentFrame();
  const pulse = interpolate(frame % 60, [0, 30, 60], [1, 1.035, 1], clamp);
  const managers = ["npm", "yarn", "pnpm", "bun", "deno"];

  return (
    <AppWindow title="alur">
      <div
        style={{
          height: "100%",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          textAlign: "center",
        }}
      >
        <div style={{ color: colors.green, fontSize: 18, textTransform: "uppercase" }}>for JavaScript projects</div>
        <div style={{ fontSize: 72, fontWeight: 900, marginTop: 12, transform: `scale(${pulse})` }}>
          alur
        </div>
        <div style={{ color: colors.paper, fontSize: 34, marginTop: 12 }}>
          one command vocabulary
        </div>
        <div style={{ display: "flex", gap: 12, marginTop: 26 }}>
          {managers.map((manager, index) => (
            <span
              key={manager}
              style={{
                opacity: appear(frame, 24 + index * 8, 12),
                border: `1px solid ${colors.line}`,
                backgroundColor: "#111111",
                padding: "10px 16px",
                color: index === 2 ? colors.green : colors.muted,
                fontSize: 18,
                ...mono,
              }}
            >
              {manager}
            </span>
          ))}
        </div>
        <div style={{ color: colors.muted, fontSize: 20, marginTop: 30, ...mono }}>
          alur.happytoolin.com
        </div>
      </div>
    </AppWindow>
  );
};

const LaunchPanel = ({
  title,
  accent,
  children,
}: {
  title: string;
  accent: Accent;
  children: ReactNode;
}) => (
  <div
    style={{
      border: `1px solid ${colors[accent]}`,
      backgroundColor: "#111111",
      padding: 20,
      boxSizing: "border-box",
      minHeight: 0,
    }}
  >
    <div style={{ color: colors[accent], fontSize: 18, marginBottom: 12, ...mono }}>{title}</div>
    {children}
  </div>
);

const LaunchBadge = ({ title, lines }: { title: string; lines: string[] }) => (
  <div
    style={{
      border: `1px solid ${colors.green}`,
      backgroundColor: "#111111",
      padding: 22,
      minHeight: 230,
      boxSizing: "border-box",
    }}
  >
    <div style={{ color: colors.green, fontSize: 22, fontWeight: 800 }}>{title}</div>
    <div style={{ marginTop: 22, display: "grid", gap: 12 }}>
      {lines.map((line) => (
        <div key={line} style={{ color: colors.paper, fontSize: 20 }}>
          {line}
        </div>
      ))}
    </div>
  </div>
);

const TaskBar = ({
  label,
  progressFrame,
  accent,
}: {
  label: string;
  progressFrame: number;
  accent: Accent;
}) => {
  const progress = interpolate(progressFrame, [0, 72], [0, 1], clamp);

  return (
    <div style={{ ...mono }}>
      <div style={{ display: "flex", justifyContent: "space-between", color: colors.paper, fontSize: 16 }}>
        <span>{label}</span>
        <span style={{ color: progress >= 1 ? colors[accent] : colors.muted }}>
          {progress >= 1 ? "done" : "running"}
        </span>
      </div>
      <div style={{ height: 8, marginTop: 8, backgroundColor: colors.line }}>
        <div style={{ width: `${progress * 100}%`, height: "100%", backgroundColor: colors[accent] }} />
      </div>
    </div>
  );
};

const FeatureStep = ({
  label,
  from,
  frame,
  accent,
}: {
  label: string;
  from: number;
  frame: number;
  accent: Accent;
}) => (
  <div
    style={{
      opacity: appear(frame, from, 12),
      transform: `translateY(${rise(frame, from, 10)}px)`,
      display: "grid",
      gridTemplateColumns: "14px 1fr",
      gap: 12,
      alignItems: "center",
      marginTop: 18,
      color: colors.paper,
      fontSize: 20,
    }}
  >
    <span
      style={{
        width: 10,
        height: 10,
        borderRadius: 999,
        backgroundColor: colors[accent],
        display: "block",
      }}
    />
    <span>{label}</span>
  </div>
);

const FastDelegateVisual = () => {
  const frame = useCurrentFrame();

  return (
    <>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 18 }}>
        <PathColumn
          title="fast when safe"
          accent="green"
          from={0}
          items={["local scripts", "project tools", "less startup time"]}
        />
        <PathColumn
          title="fallback when needed"
          accent="yellow"
          from={84}
          items={["complex workspaces", "remote packages", "exact compatibility"]}
        />
      </div>
      <div
        style={{
          position: "absolute",
          left: 28,
          right: 28,
          bottom: 28,
          opacity: appear(frame, 260, 28),
          transform: `translateY(${rise(frame, 260, 18)}px)`,
        }}
      >
        <CodeZoom
          title="same commands"
          accent="green"
          language="shell"
          lines={["$ nr --fast dev", "$ nex vitest", "$ node run dev"]}
        />
      </div>
    </>
  );
};

const PathColumn = ({
  title,
  accent,
  from,
  items,
}: {
  title: string;
  accent: Accent;
  from: number;
  items: string[];
}) => {
  const frame = useCurrentFrame();

  return (
    <div
      style={{
        opacity: appear(frame, from, 28),
        transform: `translateY(${rise(frame, from, 18)}px)`,
        border: `2px solid ${colors[accent]}`,
        padding: 16,
        backgroundColor: "#111111",
        minHeight: 244,
      }}
    >
      <div style={{ color: colors[accent], fontSize: 22, ...mono }}>{title}</div>
      <div style={{ marginTop: 16, display: "grid", gap: 12 }}>
        {items.map((item, index) => (
          <div key={item} style={{ fontSize: 17, color: index === 0 ? colors.paper : colors.muted }}>
            {item}
          </div>
        ))}
      </div>
    </div>
  );
};

const NodePassthroughVisual = () => {
  const frame = useCurrentFrame();
  const rows = [
    ["node -v", "unchanged", "cyan" as Accent],
    ["node script.js", "unchanged", "cyan" as Accent],
    ["node --watch server.js", "unchanged", "cyan" as Accent],
    ["node install vite", "package command", "green" as Accent],
    ["node run dev", "package command", "green" as Accent],
  ];

  return (
    <>
      <div style={{ fontSize: 18, color: colors.muted, textTransform: "uppercase" }}>
        normal Node still works
      </div>
      <div style={{ marginTop: 18, display: "grid", gap: 8 }}>
        {rows.map(([command, target, accent], index) => (
          <div
            key={command}
            style={{
              opacity: appear(frame, index * 42, 24),
              transform: `translateY(${rise(frame, index * 42, 14)}px)`,
              display: "grid",
              gridTemplateColumns: "1fr 106px",
              alignItems: "center",
              padding: "11px 14px",
              border: `2px solid ${colors[accent as Accent]}`,
              backgroundColor: "#111111",
              ...mono,
            }}
          >
            <span style={{ fontSize: 20, color: colors.paper }}>$ {command}</span>
            <span style={{ textAlign: "right", fontSize: 16, color: colors[accent as Accent] }}>
              {target}
            </span>
          </div>
        ))}
      </div>
      <div
        style={{
          position: "absolute",
          left: 28,
          bottom: 28,
          right: 28,
          opacity: appear(frame, 250, 24),
        }}
      >
        <CodeZoom
          title="optional package commands"
          accent="green"
          language="shell"
          lines={["node install | node run | node exec", "node x | node dlx | node ci"]}
        />
      </div>
    </>
  );
};

const CodeZoom = ({
  title,
  note,
  lines,
  accent,
  language = "text",
  style,
}: {
  title: string;
  note?: string;
  lines: string[];
  accent: Accent;
  language?: CodeLanguage;
  style?: CSSProperties;
}) => {
  const frame = useCurrentFrame();
  const scale = interpolate(frame % 120, [0, 60, 120], [1, 1.025, 1], clamp);
  const longestLine = Math.max(...lines.map((line) => line.length), title.length);
  const fontSize = lines.length > 4
    ? (longestLine > 42 ? 10 : longestLine > 34 ? 12 : 13)
    : longestLine > 38
    ? 11
    : longestLine > 30
    ? 12
    : 13;

  return (
    <div
      style={{
        border: `1px solid ${colors[accent]}`,
        backgroundColor: "#111111",
        padding: "12px 14px",
        boxSizing: "border-box",
        maxWidth: "100%",
        overflow: "hidden",
        transform: `scale(${scale})`,
        transformOrigin: "center",
        ...style,
      }}
    >
      <div
        style={{
          marginBottom: 8,
          minWidth: 0,
        }}
      >
        <span
          style={{
            color: colors[accent],
            fontSize: 13,
            whiteSpace: "nowrap",
            ...mono,
          }}
        >
          {title}
        </span>
        {note
          ? (
            <div
              style={{
                color: colors.muted,
                fontSize: 10,
                marginTop: 3,
                whiteSpace: "nowrap",
                ...mono,
              }}
            >
              {note}
            </div>
          )
          : null}
      </div>
      {lines.map((line, lineIndex) => (
        <div
          key={`${line}-${lineIndex}`}
          style={{
            fontSize,
            lineHeight: 1.28,
            whiteSpace: "pre",
            overflow: "hidden",
            textOverflow: "clip",
            ...mono,
          }}
        >
          {codeTokens(line, language).map((token, tokenIndex) => (
            <span key={`${token.text}-${tokenIndex}`} style={{ color: token.color }}>
              {token.text}
            </span>
          ))}
        </div>
      ))}
    </div>
  );
};
