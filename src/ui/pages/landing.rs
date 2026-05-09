//! Landing page component.

use leptos::prelude::*;
use leptos_meta::{Link, Meta, Title};
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::ui::auth::{AuthState, use_auth_context};
use crate::ui::common::{AvatarInitials, BrandMark, NeutralVisualPlaceholder};
use crate::ui::icon::{Icon, icons};
use crate::ui::theme::{ThemeMode, use_theme_context};

#[component]
pub fn LandingPage() -> impl IntoView {
    let auth = use_auth_context();
    let theme = use_theme_context();
    let navigate = use_navigate();

    let on_quick_start = move |_| {
        if matches!(auth.state.get(), AuthState::Authenticated(_)) {
            navigate("/dashboard", Default::default());
        } else {
            navigate("/register", Default::default());
        }
    };

    view! {
        <SeoMeta />

        <div class="app-chrome overflow-x-hidden">
            <LandingNav auth=auth theme=theme />
            <Hero on_quick_start=Callback::new(on_quick_start) />
            <HeroPreview />
            <LogoStrip />
            <StatsBar />
            <SyncSection />
            <FeatureGrid />
            <QuoteSection />
            <LandingCta />
            <LandingFooter />
        </div>
    }
}

#[component]
fn LandingNav(
    auth: crate::ui::auth::AuthContext,
    theme: crate::ui::theme::ThemeContext,
) -> impl IntoView {
    let (mobile_menu_open, set_mobile_menu_open) = signal(false);

    view! {
        <header class="chrome-header sticky">
            <A href="/" attr:class="brand-row shrink-0 hover:opacity-85 theme-transition">
                <BrandMark with_label=true version=Some("v0.9.2") />
            </A>

            <nav class="chrome-nav ml-4 hidden md:flex" aria-label="Primary navigation">
                <a href="#product" class="btn btn-ghost btn-sm">"Product"</a>
                <a href="#solutions" class="btn btn-ghost btn-sm">"Solutions"</a>
                <a href="#pricing" class="btn btn-ghost btn-sm">"Pricing"</a>
                <a href="https://github.com/c0st1nus/Archischema" target="_blank" rel="noopener noreferrer" class="btn btn-ghost btn-sm">"Docs"</a>
                <a href="https://github.com/c0st1nus/Archischema/releases" target="_blank" rel="noopener noreferrer" class="btn btn-ghost btn-sm">"Changelog"</a>
            </nav>

            <div class="flex-1"></div>

            <div class="hidden items-center gap-2 md:flex">
                <ThemeToggle theme=theme />
                <NavAuthButtons auth=auth />
            </div>

            <button
                type="button"
                class="btn-icon md:hidden"
                aria-label="Toggle mobile menu"
                aria-expanded=move || mobile_menu_open.get()
                on:click=move |_| set_mobile_menu_open.update(|open| *open = !*open)
            >
                {move || if mobile_menu_open.get() {
                    view! { <Icon name=icons::X class="h-4 w-4" /> }.into_any()
                } else {
                    view! { <Icon name=icons::MENU class="h-4 w-4" /> }.into_any()
                }}
            </button>

            <div
                class="absolute left-0 right-0 top-full border-b border-theme bg-theme-secondary p-3 shadow-theme-lg md:hidden"
                class:hidden=move || !mobile_menu_open.get()
            >
                <nav class="flex flex-col gap-2" aria-label="Mobile navigation">
                    <a href="#product" class="btn btn-ghost justify-start">"Product"</a>
                    <a href="#solutions" class="btn btn-ghost justify-start">"Solutions"</a>
                    <a href="#pricing" class="btn btn-ghost justify-start">"Pricing"</a>
                    <a href="https://github.com/c0st1nus/Archischema" target="_blank" rel="noopener noreferrer" class="btn btn-ghost justify-start">"Docs"</a>
                    <div class="hairline-h my-1"></div>
                    <ThemeToggle theme=theme />
                    <NavAuthButtons auth=auth />
                </nav>
            </div>
        </header>
    }
}

#[component]
fn NavAuthButtons(auth: crate::ui::auth::AuthContext) -> impl IntoView {
    view! {
        {move || match auth.state.get() {
            AuthState::Authenticated(_) => view! {
                <div class="flex items-center gap-2">
                    <A href="/profile" attr:class="btn btn-ghost">"Profile"</A>
                    <A href="/dashboard" attr:class="btn btn-primary">
                        "Dashboard"
                        <Icon name=icons::CHEVRON_RIGHT class="h-3 w-3" />
                    </A>
                </div>
            }.into_any(),
            _ => view! {
                <div class="flex items-center gap-2">
                    <A href="/login" attr:class="btn btn-ghost">"Sign in"</A>
                    <A href="/register" attr:class="btn btn-primary">
                        "Start free"
                        <Icon name=icons::CHEVRON_RIGHT class="h-3 w-3" />
                    </A>
                </div>
            }.into_any(),
        }}
    }
}

#[component]
fn ThemeToggle(theme: crate::ui::theme::ThemeContext) -> impl IntoView {
    view! {
        <button
            type="button"
            class="btn-icon"
            on:click=move |_| theme.toggle()
            aria-label="Toggle theme"
        >
            {move || if theme.mode.get() == ThemeMode::Dark {
                view! { <Icon name=icons::SUN class="h-4 w-4" /> }.into_any()
            } else {
                view! { <Icon name=icons::MOON class="h-4 w-4" /> }.into_any()
            }}
        </button>
    }
}

#[component]
fn Hero(on_quick_start: Callback<leptos::ev::MouseEvent>) -> impl IntoView {
    view! {
        <section class="relative mx-auto max-w-[1100px] px-7 pb-8 pt-16 text-center sm:pt-[60px]">
            <span class="chip border-theme-accent bg-theme-accent-light text-theme-accent">
                <Icon name=icons::SPARKLES class="h-3 w-3" />
                "Live now: AI schema co-pilot"
            </span>

            <h1 class="mx-auto mt-4 max-w-[780px] text-[42px] font-semibold leading-[1.05] tracking-[-0.035em] text-theme-primary sm:text-[56px]">
                "Database schemas, designed "
                <span class="text-theme-accent">"the way they're written."</span>
            </h1>

            <p class="mx-auto mt-3 max-w-[560px] text-[15px] leading-[1.55] text-theme-muted">
                "A collaborative canvas with the precision of an IDE. Visual model on the left, source of truth on the right - always in sync, with AI in between."
            </p>

            <div class="mt-6 flex flex-col items-center justify-center gap-2 sm:flex-row">
                <button type="button" class="btn btn-primary btn-lg" on:click=move |ev| on_quick_start.run(ev)>
                    "Start a diagram"
                    <Icon name=icons::CHEVRON_RIGHT class="h-3 w-3" />
                </button>
                <A href="/editor/demo" attr:class="btn btn-lg">
                    <Icon name=icons::CODE class="h-3.5 w-3.5" />
                    "Import SQL"
                </A>
            </div>

            <div class="mt-3 font-mono text-[11.5px] text-theme-muted">
                "$ npx archischema init - no credit card - open-source core"
            </div>
        </section>
    }
}

#[component]
fn HeroPreview() -> impl IntoView {
    let rows = ["users", "posts", "comments", "categories", "sessions"];

    view! {
        <section id="product" class="px-7 pb-16">
            <div class="win mx-auto max-w-[1100px]">
                <div class="win-titlebar">
                    <div class="win-traffic" aria-hidden="true">
                        <span></span><span></span><span></span>
                    </div>
                    <div class="win-url justify-center text-center">
                        "archischema.app/r/ash-spruce-12 - Blog Platform"
                    </div>
                </div>

                <div class="grid min-h-[420px] grid-cols-1 md:grid-cols-[220px_1fr]">
                    <aside class="flex flex-col gap-1 border-b border-theme p-3.5 text-xs md:border-b-0 md:border-r">
                        <div class="eyebrow px-0">"Tables - 5"</div>
                        {rows.into_iter().enumerate().map(|(index, row)| {
                            let active = row == "posts";
                            let class = if active {
                                "flex items-center gap-1.5 rounded px-1.5 py-1 font-mono text-[11.5px] bg-theme-accent-light text-theme-primary"
                            } else {
                                "flex items-center gap-1.5 rounded px-1.5 py-1 font-mono text-[11.5px] text-theme-secondary"
                            };

                            view! {
                                <div class=class>
                                    <Icon name=icons::DATABASE class=if active { "h-3 w-3 text-theme-accent" } else { "h-3 w-3 text-theme-muted" } />
                                    <span>{row}</span>
                                    <span class="ml-auto text-[10.5px] text-theme-muted">{(index + 3).to_string()}</span>
                                </div>
                            }
                        }).collect_view()}
                    </aside>

                    <div class="relative min-h-[360px] overflow-hidden bg-theme-canvas p-3">
                        <div class="absolute left-3 top-3 z-10 flex gap-1">
                            <span class="chip">
                                <span class="h-1.5 w-1.5 rounded-full bg-theme-accent"></span>
                                "Alice - Bob - Constantine"
                            </span>
                        </div>
                        <NeutralVisualPlaceholder label="Collaborative workspace".to_string() class="h-full min-h-[330px]".to_string() />
                    </div>
                </div>
            </div>
        </section>
    }
}

#[component]
fn LogoStrip() -> impl IntoView {
    let brands = [
        ("sigma", false),
        ("NORTHWIND", true),
        ("helix/db", true),
        ("Plinth.io", false),
        ("OBSERVABLE", false),
        ("acme.run", true),
    ];

    view! {
        <section class="mx-auto max-w-[1100px] px-7 pb-14">
            <div class="mb-4 text-center font-mono text-[11.5px] uppercase tracking-[0.12em] text-theme-muted">
                "Schemas designed at"
            </div>
            <div class="grid grid-cols-2 gap-2 border-y border-theme py-4 sm:grid-cols-3 lg:grid-cols-6">
                {brands.into_iter().map(|(brand, mono)| {
                    let class = if mono {
                        "flex h-8 items-center justify-center font-mono text-sm font-medium tracking-[-0.01em] text-theme-muted opacity-70"
                    } else {
                        "flex h-8 items-center justify-center text-sm font-medium text-theme-muted opacity-70"
                    };

                    view! { <div class=class>{brand}</div> }
                }).collect_view()}
            </div>
        </section>
    }
}

#[component]
fn StatsBar() -> impl IntoView {
    let stats = [
        ("24k", "diagrams modeled", true),
        ("3.2M", "columns under management", false),
        ("42ms", "sync latency - p95", false),
        ("100%", "local-first - CRDT", false),
    ];

    view! {
        <section id="solutions" class="mx-auto max-w-[1100px] px-7 pb-16">
            <div class="grid overflow-hidden rounded-xl border border-theme bg-theme-secondary sm:grid-cols-2 lg:grid-cols-4">
                {stats.into_iter().enumerate().map(|(index, (value, label, accent))| {
                    let border_class = if index < 3 { "border-b border-theme p-5 lg:border-b-0 lg:border-r" } else { "p-5" };
                    let value_class = if accent {
                        "text-[28px] font-semibold tracking-[-0.025em] text-theme-accent"
                    } else {
                        "text-[28px] font-semibold tracking-[-0.025em] text-theme-primary"
                    };

                    view! {
                        <div class=border_class>
                            <div class=value_class>{value}</div>
                            <div class="mt-1 text-xs text-theme-muted">{label}</div>
                        </div>
                    }
                }).collect_view()}
            </div>
        </section>
    }
}

#[component]
fn SyncSection() -> impl IntoView {
    let lines = [
        ("1", "CREATE TABLE posts (", "text-theme-primary", false),
        (
            "2",
            "  id          uuid PRIMARY KEY,",
            "text-theme-secondary",
            false,
        ),
        (
            "3",
            "  user_id     uuid NOT NULL REFERENCES users(id),",
            "text-theme-secondary",
            false,
        ),
        (
            "4",
            "  title       varchar(180) NOT NULL,",
            "text-theme-secondary",
            false,
        ),
        (
            "5",
            "+ category_id uuid REFERENCES categories(id)",
            "text-theme-accent",
            true,
        ),
        ("6", "+   ON DELETE SET NULL,", "text-theme-accent", true),
        ("7", "  body        text,", "text-theme-secondary", false),
        (
            "8",
            "  created_at  timestamptz DEFAULT now()",
            "text-theme-secondary",
            false,
        ),
        ("9", ");", "text-theme-primary", false),
    ];

    view! {
        <section class="mx-auto max-w-[1100px] px-7 pb-20">
            <div class="eyebrow px-0">"One source of truth"</div>
            <h2 class="mt-1 text-[32px] font-semibold tracking-[-0.02em] text-theme-primary">
                "Edit either side. Both stay canonical."
            </h2>
            <p class="mt-2 max-w-[580px] text-sm leading-[1.55] text-theme-muted">
                "Move between canvas thinking and SQL precision without a code-gen step or a brittle round-trip."
            </p>

            <div class="mt-6 grid overflow-hidden rounded-xl border border-theme bg-theme-secondary lg:grid-cols-2">
                <div class="border-b border-theme lg:border-b-0 lg:border-r">
                    <div class="flex items-center gap-2 border-b border-theme px-3.5 py-2.5 font-mono text-[11.5px] text-theme-muted">
                        <Icon name=icons::CODE class="h-3 w-3 text-theme-accent" />
                        "schema.sql"
                        <span class="chip ml-auto h-[18px] border-theme-accent text-[10px] text-theme-accent">"+2 lines"</span>
                    </div>
                    <div class="py-3 font-mono text-xs leading-[1.7]">
                        {lines.into_iter().map(|(number, text, text_class, highlight)| {
                            let row_class = if highlight {
                                "grid grid-cols-[36px_1fr] bg-theme-accent-light"
                            } else {
                                "grid grid-cols-[36px_1fr]"
                            };
                            view! {
                                <div class=row_class>
                                    <span class="pr-2.5 text-right text-theme-muted opacity-60">{number}</span>
                                    <span class=format!("whitespace-pre {}", text_class)>{text}</span>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                </div>

                <div class="min-h-[280px] bg-theme-canvas p-4">
                    <NeutralVisualPlaceholder label="Canonical visual state".to_string() class="h-full min-h-[248px]".to_string() />
                </div>
            </div>
        </section>
    }
}

#[component]
fn FeatureGrid() -> impl IntoView {
    let features = [
        (
            icons::CODE,
            "Two-way SQL",
            "Edit visually or in source. Both stay canonical, with diff before apply.",
        ),
        (
            icons::SPARKLES,
            "AI schema actions",
            "Ask for audit columns, indexes, or cleanups and review the generated changes.",
        ),
        (
            icons::USER,
            "CRDT collaboration",
            "Real-time sessions with presence, stable IDs, and conflict-safe edits.",
        ),
        (
            icons::WARNING,
            "Inline diagnostics",
            "Catch naming drift, missing constraints, and risky model changes earlier.",
        ),
        (
            icons::ARROW_DOWN_TO_LINE,
            "Export anywhere",
            "Postgres-first SQL plus JSON export for pipelines and review workflows.",
        ),
        (
            icons::KEY,
            "Bring your own model",
            "OpenRouter-compatible AI proxy with explicit modes and tool boundaries.",
        ),
    ];

    view! {
        <section class="mx-auto max-w-[1100px] px-7 pb-16">
            <div class="eyebrow px-0">"Built for serious data work"</div>
            <h2 class="mb-7 mt-1 text-[32px] font-semibold tracking-[-0.02em] text-theme-primary">
                "A canvas that reads the docs."
            </h2>
            <div class="grid gap-3.5 md:grid-cols-2 lg:grid-cols-3">
                {features.into_iter().map(|(icon, title, body)| view! {
                    <article class="surface p-4">
                        <div class="mb-2.5 flex h-[30px] w-[30px] items-center justify-center rounded-lg bg-theme-accent-light text-theme-accent">
                            <Icon name=icon class="h-4 w-4" />
                        </div>
                        <h3 class="text-[13.5px] font-semibold text-theme-primary">{title}</h3>
                        <p class="mt-1 text-xs leading-normal text-theme-muted">{body}</p>
                    </article>
                }).collect_view()}
            </div>
        </section>
    }
}

#[component]
fn QuoteSection() -> impl IntoView {
    view! {
        <section class="mx-auto max-w-[880px] px-7 pb-20">
            <div class="surface relative p-8">
                <div class="absolute -top-3 left-6 rounded-full border border-theme bg-theme-secondary px-2.5 py-0.5 font-mono text-[10.5px] uppercase tracking-[0.1em] text-theme-muted">
                    "From the field"
                </div>
                <blockquote class="max-w-[720px] text-[22px] font-medium leading-[1.4] tracking-[-0.015em] text-theme-primary">
                    "We replaced three tools with ArchiSchema - the diagram, the migration plan, and "
                    <span class="text-theme-accent">"the conversation about whether the diagram is right."</span>
                    " Source and visual finally agree."
                </blockquote>
                <div class="mt-5 flex items-center gap-2.5">
                    <AvatarInitials name="Marta Yi".to_string() color="oklch(0.62 0.16 185)".to_string() />
                    <div>
                        <div class="text-[13px] font-medium text-theme-primary">"Marta Yi"</div>
                        <div class="font-mono text-[11.5px] text-theme-muted">"Staff data eng - helix/db"</div>
                    </div>
                </div>
            </div>
        </section>
    }
}

#[component]
fn LandingCta() -> impl IntoView {
    view! {
        <section id="pricing" class="px-7 pb-16">
            <div class="mx-auto max-w-[1100px] overflow-hidden rounded-2xl border border-theme-accent bg-theme-secondary p-8 sm:p-10">
                <div class="eyebrow px-0 text-theme-accent">"v0.9.2 - public beta"</div>
                <h2 class="mt-1 max-w-[640px] text-4xl font-semibold tracking-[-0.025em] text-theme-primary">
                    "Stop drawing diagrams that lie."
                </h2>
                <p class="mt-2 max-w-[520px] text-sm leading-[1.55] text-theme-muted">
                    "Free for solo work. Free forever for OSS. Pay only when you bring a team."
                </p>
                <div class="mt-6 flex flex-col gap-2 sm:flex-row sm:items-center">
                    <A href="/register" attr:class="btn btn-primary btn-lg">
                        "Start a diagram"
                        <Icon name=icons::CHEVRON_RIGHT class="h-3 w-3" />
                    </A>
                    <a href="https://github.com/c0st1nus/Archischema" target="_blank" rel="noopener noreferrer" class="btn btn-lg">
                        <Icon name=icons::GITHUB class="h-3.5 w-3.5" />
                        "Read the docs"
                    </a>
                    <span class="font-mono text-[11.5px] text-theme-muted sm:ml-1.5">
                        "Cmd + K to open the command bar from anywhere"
                    </span>
                </div>
            </div>
        </section>
    }
}

#[component]
fn LandingFooter() -> impl IntoView {
    let columns = [
        (
            "Product",
            [
                "Editor",
                "Source mode",
                "AI assistant",
                "LiveShare",
                "Changelog",
            ],
        ),
        (
            "Resources",
            [
                "Documentation",
                "Templates",
                "Postgres guide",
                "API reference",
                "GitHub",
            ],
        ),
        (
            "Company",
            ["About", "Careers", "Press kit", "Contact", "Status"],
        ),
        (
            "Legal",
            ["Privacy", "Terms", "Security", "Licenses", "Cookies"],
        ),
    ];

    view! {
        <footer class="mx-auto w-full max-w-[1100px] border-t border-theme px-7 pb-7 pt-9">
            <div class="grid gap-8 pb-6 md:grid-cols-[1.4fr_repeat(4,1fr)]">
                <div>
                    <div class="mb-2.5">
                        <BrandMark with_label=true />
                    </div>
                    <p class="max-w-[280px] text-xs leading-[1.55] text-theme-muted">
                        "A canvas with the precision of an IDE. Made for the Postgres era."
                    </p>
                    <div class="mt-3.5 flex gap-1.5">
                        {[("GH", "https://github.com/c0st1nus/Archischema"), ("X", "#"), ("DC", "#")].into_iter().map(|(label, href)| view! {
                            <a href=href class="flex h-[26px] w-[26px] items-center justify-center rounded-md border border-theme bg-theme-secondary font-mono text-[10px] text-theme-muted hover:text-theme-primary">
                                {label}
                            </a>
                        }).collect_view()}
                    </div>
                </div>

                {columns.into_iter().map(|(title, items)| view! {
                    <div>
                        <div class="eyebrow mb-2.5 px-0">{title}</div>
                        <div class="flex flex-col gap-1.5">
                            {items.into_iter().map(|item| view! {
                                <a href="#" class="text-xs text-theme-muted hover:text-theme-primary">{item}</a>
                            }).collect_view()}
                        </div>
                    </div>
                }).collect_view()}
            </div>

            <div class="flex flex-col gap-3 border-t border-theme pt-4 font-mono text-[11px] text-theme-muted sm:flex-row sm:items-center sm:justify-between">
                <span>"(c) 2026 ArchiSchema - v0.9.2"</span>
                <span class="flex items-center gap-1.5">
                    <span class="h-1.5 w-1.5 rounded-full bg-theme-accent"></span>
                    "All systems operational"
                </span>
            </div>
        </footer>
    }
}

#[component]
fn SeoMeta() -> impl IntoView {
    view! {
        <Title text="Archischema - Visual Database Schema Designer" />
        <Meta name="description" content="Design database schemas with an IDE-precise collaborative canvas, SQL source mode, LiveShare, and AI assistance." />
        <Meta name="keywords" content="database schema, schema designer, ERD, database design, SQL, visual editor, collaboration, AI assistant" />
        <Meta property="og:type" content="website" />
        <Meta property="og:url" content="https://archischema.io/" />
        <Meta property="og:title" content="Archischema - Visual Database Schema Designer" />
        <Meta property="og:description" content="A collaborative canvas with the precision of an IDE. Visual model and SQL source stay in sync." />
        <Meta property="og:image" content="https://archischema.io/og-image.png" />
        <Meta property="twitter:card" content="summary_large_image" />
        <Meta property="twitter:url" content="https://archischema.io/" />
        <Meta property="twitter:title" content="Archischema - Visual Database Schema Designer" />
        <Meta property="twitter:description" content="A collaborative canvas with the precision of an IDE. Visual model and SQL source stay in sync." />
        <Meta property="twitter:image" content="https://archischema.io/og-image.png" />
        <Link rel="canonical" href="https://archischema.io/" />
        <script type="application/ld+json" inner_html=r#"{"@context":"https://schema.org","@type":"SoftwareApplication","name":"Archischema","applicationCategory":"DeveloperApplication","operatingSystem":"Web","description":"Visual database schema designer with real-time collaboration and AI assistance","url":"https://archischema.io","author":{"@type":"Organization","name":"Archischema"},"offers":{"@type":"Offer","price":"0","priceCurrency":"USD"},"featureList":["Visual schema editor","SQL source mode","Real-time collaboration","AI-powered schema generation","SQL and JSON export"]}"#></script>
    }
}
