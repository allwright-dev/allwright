const statusItems = [
  { label: "Vision", value: "One engine for every automation flow" },
  { label: "Promise", value: "One system instead of a pile of brittle tools" },
  { label: "Launch", value: "Public release coming soon" },
];

export default function Home() {
  return (
    <main className="relative grid min-h-screen place-items-center overflow-hidden bg-[radial-gradient(circle_at_top_left,rgba(216,106,49,0.22),transparent_34%),radial-gradient(circle_at_85%_20%,rgba(35,105,91,0.2),transparent_26%),linear-gradient(180deg,var(--background)_0%,var(--background-deep)_100%)] px-4 py-8 sm:px-6">
      <div className="grid-overlay pointer-events-none absolute inset-0 opacity-25" />
      <div className="ambient-left absolute left-[-8%] top-[8%] h-[22rem] w-[22rem] rounded-full bg-[radial-gradient(circle,rgba(216,106,49,0.28),transparent_68%)] blur-md" />
      <div className="ambient-right absolute bottom-[2%] right-[-8%] h-[22rem] w-[22rem] rounded-full bg-[radial-gradient(circle,rgba(35,105,91,0.24),transparent_68%)] blur-md" />

      <section className="animate-rise relative w-full max-w-6xl rounded-[2rem] border border-[var(--line)] bg-[var(--card)] p-6 shadow-[0_28px_80px_rgba(32,26,17,0.12)] backdrop-blur-xl sm:p-10">
        <p className="mb-5 font-mono text-[0.82rem] uppercase tracking-[0.18em] text-[var(--accent-2)]">
          allwright.dev is getting ready
        </p>
        <h1 className="max-w-[12ch] text-[clamp(3rem,8vw,6.8rem)] leading-[0.95] font-semibold tracking-[-0.06em] text-[var(--ink)]">
          One automation engine,
          <span className="text-[color:rgba(29,34,28,0.72)]">
            {" "}
            for everything you need to automate.
          </span>
        </h1>
        <p className="mt-6 max-w-[56ch] text-[clamp(1rem,2.2vw,1.2rem)] leading-7 text-[var(--muted)] sm:leading-8">
          Allwright is being built around a simple idea: automation should feel
          unified. Instead of stitching together separate tools for browser
          tasks, workflows, and repetitive operations, you should be able to
          rely on one engine that handles it all.
        </p>

        <div
          aria-label="project status"
          className="mt-9 grid gap-4 md:grid-cols-3"
        >
          {statusItems.map((item) => (
            <article
              key={item.label}
              className="rounded-[1.375rem] border border-[var(--line)] bg-white/55 px-4 py-5"
            >
              <p className="mb-2.5 font-mono text-[0.78rem] uppercase tracking-[0.08em] text-[var(--muted)]">
                {item.label}
              </p>
              <strong className="block text-base leading-6 text-[var(--ink)]">
                {item.value}
              </strong>
            </article>
          ))}
        </div>

        <div className="mt-4 grid gap-4 md:grid-cols-[1.1fr_0.9fr]">
          <div className="rounded-[1.375rem] border border-[var(--line)] bg-white/55 px-5 py-5">
            <span className="mb-2.5 inline-block font-mono text-[0.8rem] uppercase tracking-[0.08em] text-[var(--accent)]">
              Why allwright
            </span>
            <p className="text-[var(--muted)] leading-7">
              The name says it plainly: one engine designed to make every kind
              of automation feel all right in one place, instead of scattered
              across disconnected systems.
            </p>
          </div>
          <div className="rounded-[1.375rem] border border-[var(--line)] bg-white/55 px-5 py-5">
            <span className="mb-2.5 inline-block font-mono text-[0.8rem] uppercase tracking-[0.08em] text-[var(--accent)]">
              What is coming
            </span>
            <p className="text-[var(--muted)] leading-7">
              A clearer product story, launch updates, examples, and a public
              home for the platform are on the way.
            </p>
          </div>
        </div>
      </section>
    </main>
  );
}
