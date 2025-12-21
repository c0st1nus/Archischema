//! Landing page component
//!
//! An immersive, scroll-animated landing page for Archischema featuring:
//! - SEO meta tags for search engine optimization
//! - Hero section with quick start and demo buttons
//! - Features section with benefit cards
//! - Visual Editor section with animated cursor dragging a table
//! - Real-time Collaboration section with two cursors creating and connecting tables
//! - AI Assistant section with realistic interface matching the actual editor
//! - Pricing section with plan comparison
//! - FAQ section with accordion
//! - Call-to-action and footer sections

use leptos::prelude::*;
use leptos_meta::{Link, Meta, Title};
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::ui::auth::{AuthState, use_auth_context};
use crate::ui::icon::{Icon, icons};
use crate::ui::theme::{ThemeMode, use_theme_context};

/// Landing page component with scroll-based animations
#[component]
pub fn LandingPage() -> impl IntoView {
    let auth = use_auth_context();
    let theme = use_theme_context();
    let navigate = use_navigate();

    // Quick Start button handler
    let on_quick_start = move |_| {
        if matches!(auth.state.get(), AuthState::Authenticated(_)) {
            navigate("/dashboard", Default::default());
        } else {
            navigate("/login", Default::default());
        }
    };

    view! {
        // SEO Meta Tags
        <SeoMeta />

        <div class="min-h-screen bg-theme-primary overflow-x-hidden">
            <Header theme=theme auth=auth />

            // Hero Section
            <section class="min-h-screen flex items-center justify-center relative pt-16">
                <div class="text-center px-4 max-w-4xl mx-auto">
                    <h1 class="text-5xl sm:text-6xl lg:text-7xl font-bold text-theme-primary mb-6 tracking-tight
                               landing-fade-in-up">
                        "Archischema"
                    </h1>
                    <p class="text-xl sm:text-2xl text-theme-secondary max-w-2xl mx-auto mb-10 leading-relaxed
                              landing-fade-in-up landing-delay-200">
                        "Design beautiful database schemas visually. Collaborate in real-time. Let AI help you build faster."
                    </p>

                    <div class="flex flex-col sm:flex-row items-center justify-center gap-4 landing-fade-in-up landing-delay-400">
                        <button
                            class="landing-btn-primary"
                            on:click=on_quick_start
                            aria-label="Get started with Archischema"
                        >
                            "Quick Start"
                        </button>
                        <A
                            href="/editor/demo"
                            attr:class="landing-btn-secondary"
                            attr:aria-label="Try the demo editor"
                        >
                            "Try Demo"
                        </A>
                    </div>

                    // Scroll indicator
                    <div class="absolute bottom-8 left-1/2 -translate-x-1/2 animate-bounce">
                        <Icon name=icons::CHEVRON_DOWN class="w-6 h-6 text-theme-tertiary" />
                    </div>
                </div>

                // Background decoration
                <div class="absolute inset-0 -z-10 overflow-hidden" aria-hidden="true">
                    <div class="absolute top-1/4 left-1/4 w-96 h-96 bg-accent-primary/5 rounded-full blur-3xl"></div>
                    <div class="absolute bottom-1/4 right-1/4 w-96 h-96 bg-blue-500/5 rounded-full blur-3xl"></div>
                </div>
            </section>

            // Features Section
            <section class="py-20 px-4 bg-theme-secondary/10">
                <div class="max-w-6xl mx-auto">
                    <div class="text-center mb-16 landing-scroll-animate">
                        <h2 class="text-3xl sm:text-4xl font-bold text-theme-primary mb-4">
                            "Why Archischema?"
                        </h2>
                        <p class="text-lg text-theme-secondary max-w-2xl mx-auto">
                            "Everything you need to design, collaborate, and export database schemas efficiently."
                        </p>
                    </div>

                    <div class="grid md:grid-cols-3 gap-8">
                        <FeatureCard
                            icon="visual"
                            title="Visual Design"
                            description="Intuitive drag-and-drop interface. No SQL knowledge required to get started."
                        />
                        <FeatureCard
                            icon="collab"
                            title="Real-time Collaboration"
                            description="Work together with your team. See changes instantly with live cursors."
                        />
                        <FeatureCard
                            icon="ai"
                            title="AI-Powered"
                            description="Generate schemas from descriptions. Get optimization suggestions automatically."
                        />
                        <FeatureCard
                            icon="export"
                            title="Multiple Exports"
                            description="Export to SQL, JSON, or connect directly to your database."
                        />
                        <FeatureCard
                            icon="version"
                            title="Version History"
                            description="Track changes over time. Restore previous versions with one click."
                        />
                        <FeatureCard
                            icon="secure"
                            title="Secure by Default"
                            description="Your data is encrypted. Self-host or use our cloud with confidence."
                        />
                    </div>
                </div>
            </section>

            // Visual Editor Section
            <section class="min-h-screen flex items-center justify-center py-20 px-4">
                <div class="max-w-6xl mx-auto w-full">
                    <div class="text-center mb-16 landing-scroll-animate">
                        <h2 class="text-4xl sm:text-5xl font-bold text-theme-primary mb-4">
                            "Visual Editor"
                        </h2>
                        <p class="text-xl text-theme-secondary max-w-2xl mx-auto">
                            "Drag and drop tables, define columns, and create relationships with an intuitive visual interface."
                        </p>
                    </div>

                    // Animated canvas
                    <div class="relative h-96 bg-theme-secondary/30 rounded-2xl border border-theme overflow-hidden landing-scroll-animate">
                        // Grid background
                        <div class="absolute inset-0 opacity-20 landing-grid-bg" aria-hidden="true"></div>

                        // Container for table and cursor that move together
                        <div class="absolute landing-visual-editor-container">
                            // Animated cursor on the drag handle
                            <div class="absolute top-3 left-[170px] z-10">
                                <AnimatedCursor color="#3b82f6" label="You" />
                            </div>
                            <AnimatedTable name="users" />
                        </div>
                    </div>
                </div>
            </section>

            // Real-time Collaboration Section
            <section class="min-h-screen flex items-center justify-center py-20 px-4 bg-theme-secondary/20">
                <div class="max-w-6xl mx-auto w-full">
                    <div class="text-center mb-16 landing-scroll-animate">
                        <h2 class="text-4xl sm:text-5xl font-bold text-theme-primary mb-4">
                            "Real-Time Collaboration"
                        </h2>
                        <p class="text-xl text-theme-secondary max-w-2xl mx-auto">
                            "Work together with your team in real-time. See changes instantly and never worry about conflicts."
                        </p>
                    </div>

                    // Animated collaboration canvas
                    <div class="relative h-96 bg-theme-secondary/30 rounded-2xl border border-theme overflow-hidden landing-scroll-animate">
                        // Grid background
                        <div class="absolute inset-0 opacity-20 landing-grid-bg" aria-hidden="true"></div>

                        // SVG relation line
                        <svg class="absolute inset-0 w-full h-full pointer-events-none landing-collab-connection" viewBox="0 0 1152 384" preserveAspectRatio="none" style="z-index: 1;" aria-hidden="true">
                            <defs>
                                <marker
                                    id="arrowhead-landing"
                                    markerWidth="10"
                                    markerHeight="7"
                                    refX="9"
                                    refY="3.5"
                                    orient="auto"
                                >
                                    <polygon points="0 0, 10 3.5, 0 7" fill="#6b7280" />
                                </marker>
                            </defs>
                            <path
                                d="M 220 205 L 605 205 L 605 135 L 918 135"
                                stroke="#6b7280"
                                stroke-width="2"
                                fill="none"
                                marker-end="url(#arrowhead-landing)"
                            />
                            <text x="685" y="115" fill="#6b7280" font-size="14" font-weight="500" text-anchor="middle">"N:1"</text>
                        </svg>

                        // Table 1 (left - posts)
                        <div class="absolute top-1/2 left-[3%] -translate-y-1/2 landing-collab-table-left" style="z-index: 2;">
                            <div class="absolute top-3 left-[170px] z-10 landing-cursor-alice">
                                <AnimatedCursor color="#3b82f6" label="Alice" />
                            </div>
                            <AnimatedTable name="posts" />
                        </div>

                        // Table 2 (right - users)
                        <div class="absolute top-[35%] right-[3%] -translate-y-1/2 landing-collab-table-right" style="z-index: 2;">
                            <div class="absolute top-3 left-[130px] z-10 landing-cursor-bob">
                                <AnimatedCursor color="#10b981" label="Bob" />
                            </div>
                            <AnimatedTable name="users_small" />
                        </div>
                    </div>
                </div>
            </section>

            // AI Assistant Section
            <section class="min-h-screen flex items-center justify-center py-20 px-4">
                <div class="max-w-6xl mx-auto w-full">
                    <div class="text-center mb-16 landing-scroll-animate">
                        <h2 class="text-4xl sm:text-5xl font-bold text-theme-primary mb-4">
                            "AI Assistant"
                        </h2>
                        <p class="text-xl text-theme-secondary max-w-2xl mx-auto">
                            "Let AI help you design better schemas. Generate tables from descriptions and get optimization suggestions."
                        </p>
                    </div>

                    <div class="max-w-4xl mx-auto landing-ai-demo landing-scroll-animate">
                        <AIInterfaceMockup />
                    </div>
                </div>
            </section>

            // Pricing Section
            <PricingSection />

            // FAQ Section
            <FaqSection />

            // CTA Section
            <section class="py-24 px-4 bg-gradient-to-b from-transparent to-theme-secondary/30">
                <div class="max-w-4xl mx-auto text-center landing-scroll-animate">
                    <h2 class="text-3xl sm:text-4xl font-bold text-theme-primary mb-4">
                        "Ready to build your schema?"
                    </h2>
                    <p class="text-lg text-theme-secondary mb-8 max-w-xl mx-auto">
                        "Join developers who design their databases visually with Archischema."
                    </p>
                    <div class="flex flex-col sm:flex-row items-center justify-center gap-4">
                        <A
                            href="/register"
                            attr:class="landing-btn-primary"
                        >
                            "Get Started Free"
                        </A>
                        <a
                            href="https://github.com/c0st1nus/Archischema"
                            target="_blank"
                            rel="noopener noreferrer"
                            class="landing-btn-secondary inline-flex items-center gap-2"
                            aria-label="View project on GitHub"
                        >
                            <GithubIcon />
                            "View on GitHub"
                        </a>
                    </div>
                </div>
            </section>

            // Footer
            <Footer />

            // CSS Animations
            <LandingStyles />

            // Intersection Observer for scroll animations
            <ScrollAnimationScript />
        </div>
    }
}

/// Header component with mobile menu support
#[component]
fn Header(
    theme: crate::ui::theme::ThemeContext,
    auth: crate::ui::auth::AuthContext,
) -> impl IntoView {
    let (mobile_menu_open, set_mobile_menu_open) = signal(false);

    view! {
        <header class="fixed top-0 left-0 right-0 z-50 bg-theme-primary/80 backdrop-blur-md border-b border-theme/50">
            <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
                <div class="flex items-center justify-between h-16">
                    // Logo
                    <A href="/" attr:class="flex items-center gap-3 hover:opacity-80 transition-opacity">
                        <Logo />
                        <span class="text-xl font-bold text-theme-primary">"Archischema"</span>
                    </A>

                    // Desktop Navigation
                    <div class="hidden md:flex items-center gap-6">
                        <nav class="flex items-center gap-4">
                            <a href="#pricing" class="text-sm font-medium text-theme-secondary hover:text-theme-primary transition-colors">
                                "Pricing"
                            </a>
                            <a href="#faq" class="text-sm font-medium text-theme-secondary hover:text-theme-primary transition-colors">
                                "FAQ"
                            </a>
                            <AuthButtons auth=auth />
                        </nav>
                        <ThemeToggle theme=theme />
                    </div>

                    // Mobile menu button
                    <button
                        class="md:hidden p-2 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
                        on:click=move |_| set_mobile_menu_open.update(|v| *v = !*v)
                        aria-label="Toggle mobile menu"
                        aria-expanded=move || mobile_menu_open.get()
                    >
                        {move || {
                            if mobile_menu_open.get() {
                                view! {
                                    <Icon name=icons::X class="w-6 h-6 text-theme-primary" />
                                }.into_any()
                            } else {
                                view! {
                                    <Icon name=icons::MENU class="w-6 h-6 text-theme-primary" />
                                }.into_any()
                            }
                        }}
                    </button>
                </div>

                // Mobile menu
                <div
                    class="md:hidden overflow-hidden transition-all duration-300"
                    class:max-h-0=move || !mobile_menu_open.get()
                    class:max-h-96=move || mobile_menu_open.get()
                >
                    <div class="py-4 space-y-4 border-t border-theme/50">
                        <nav class="flex flex-col gap-2">
                            <a
                                href="#pricing"
                                class="block px-4 py-2 text-sm font-medium text-theme-secondary hover:text-theme-primary hover:bg-theme-secondary/30 rounded-lg transition-colors"
                                on:click=move |_| set_mobile_menu_open.set(false)
                            >
                                "Pricing"
                            </a>
                            <a
                                href="#faq"
                                class="block px-4 py-2 text-sm font-medium text-theme-secondary hover:text-theme-primary hover:bg-theme-secondary/30 rounded-lg transition-colors"
                                on:click=move |_| set_mobile_menu_open.set(false)
                            >
                                "FAQ"
                            </a>
                            <MobileAuthButtons auth=auth />
                            <ThemeToggle theme=theme />
                        </nav>


                    </div>
                </div>
            </div>
        </header>
    }
}

/// Theme toggle button component
#[component]
fn ThemeToggle(theme: crate::ui::theme::ThemeContext) -> impl IntoView {
    view! {
        <button
            class="p-2 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors text-gray-600 dark:text-gray-300
                   border border-gray-300 dark:border-gray-600"
            on:click=move |_| theme.toggle()
            aria-label="Toggle theme"
        >
            {move || {
                if theme.mode.get() == ThemeMode::Dark {
                    view! {
                        <Icon name=icons::SUN class="w-5 h-5" />
                    }
                } else {
                    view! {
                        <Icon name=icons::MOON class="w-5 h-5" />
                    }
                }
            }}
        </button>
    }
}

/// Desktop auth buttons with dropdown for authenticated users
#[component]
fn AuthButtons(auth: crate::ui::auth::AuthContext) -> impl IntoView {
    let dropdown_open = RwSignal::new(false);

    view! {
        {move || {
            match auth.state.get() {
                AuthState::Authenticated(user) => {
                    // Generate avatar color from username
                    let hash = user.username.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32));
                    let colors = [
                        "bg-blue-500", "bg-green-500", "bg-yellow-500", "bg-red-500",
                        "bg-purple-500", "bg-pink-500", "bg-indigo-500", "bg-teal-500",
                    ];
                    let color = colors[(hash as usize) % colors.len()];
                    let initials = user.username.chars().next().unwrap_or('?').to_uppercase().to_string();
                    let user_clone = user.clone();

                    view! {
                        <div class="relative">
                            // Avatar button
                            <button
                                class="flex items-center gap-2 p-1 rounded-lg hover:bg-theme-secondary transition-colors"
                                on:click=move |_| dropdown_open.update(|v| *v = !*v)
                            >
                                <div class=format!("w-8 h-8 rounded-full flex items-center justify-center text-white font-medium text-sm {}", color)>
                                    {initials.clone()}
                                </div>
                                <div
                                    class="flex items-center justify-center h-4 w-4 text-theme-tertiary transition-transform duration-200"
                                    class=("rotate-180", move || dropdown_open.get())
                                >
                                    <Icon name=icons::CHEVRON_DOWN class="h-4 w-4" />
                                </div>
                            </button>

                            // Dropdown menu
                            {move || {
                                if dropdown_open.get() {
                                    let user_inner = user_clone.clone();
                                    Some(view! {
                                        <div class="absolute right-0 mt-2 w-56 bg-theme-primary rounded-lg shadow-lg border border-theme py-1 z-50">
                                            // User info header
                                            <div class="px-4 py-3 border-b border-theme">
                                                <p class="text-sm font-medium text-theme-primary truncate">
                                                    {user_inner.username.clone()}
                                                </p>
                                                <p class="text-xs text-theme-tertiary truncate">
                                                    {user_inner.email.clone()}
                                                </p>
                                            </div>

                                            // Menu items
                                            <div class="py-1">
                                                <A
                                                    href="/profile"
                                                    attr:class="w-full px-4 py-2 text-sm text-left text-theme-primary
                                                           hover:bg-theme-secondary transition-colors flex items-center gap-2"
                                                >
                                                    <Icon name=icons::USER class="h-4 w-4" />
                                                    "Profile"
                                                </A>
                                                <A
                                                    href="/dashboard"
                                                    attr:class="w-full px-4 py-2 text-sm text-left text-theme-primary
                                                           hover:bg-theme-secondary transition-colors flex items-center gap-2"
                                                >
                                                    <Icon name=icons::SQUARES_2X2 class="h-4 w-4" />
                                                    "My Diagrams"
                                                </A>
                                            </div>

                                            // Divider
                                            <div class="border-t border-theme my-1"></div>

                                            // Sign out
                                            <div class="py-1">
                                                <button
                                                    class="w-full px-4 py-2 text-sm text-left text-red-500
                                                           hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors
                                                           flex items-center gap-2"
                                                    on:click=move |_| {
                                                        dropdown_open.set(false);
                                                        leptos::task::spawn_local(async move {
                                                            crate::ui::auth::logout().await;
                                                        });
                                                    }
                                                >
                                                    <Icon name=icons::LOGOUT class="h-4 w-4" />
                                                    "Sign Out"
                                                </button>
                                            </div>
                                        </div>
                                    })
                                } else {
                                    None
                                }
                            }}
                        </div>
                    }.into_any()
                }
                _ => {
                    view! {
                        <div class="flex items-center gap-2">
                            <A
                                href="/login"
                                attr:class="px-4 py-2 text-sm font-medium text-gray-600 dark:text-gray-300 hover:text-gray-900 dark:hover:text-white transition-colors"
                            >
                                "Sign In"
                            </A>
                            <A
                                href="/register"
                                attr:class="px-4 py-2 text-sm font-medium text-white bg-accent-primary hover:bg-accent-primary-hover rounded-lg transition-colors shadow-md"
                            >
                                "Sign Up"
                            </A>
                        </div>
                    }.into_any()
                }
            }
        }}
    }
}

/// Mobile auth buttons
#[component]
fn MobileAuthButtons(auth: crate::ui::auth::AuthContext) -> impl IntoView {
    view! {
        {move || {
            match auth.state.get() {
                AuthState::Authenticated(user) => {
                    view! {
                        <div class="space-y-2">
                            <div class="px-4 py-2 border-b border-theme">
                                <p class="text-sm font-medium text-theme-primary">{user.username.clone()}</p>
                                <p class="text-xs text-theme-tertiary">{user.email.clone()}</p>
                            </div>
                            <A
                                href="/profile"
                                attr:class="block w-full text-center px-4 py-2 text-sm font-medium text-theme-primary border border-theme rounded-lg"
                            >
                                "Profile"
                            </A>
                            <A
                                href="/dashboard"
                                attr:class="block w-full text-center px-4 py-2 text-sm font-medium text-white bg-accent-primary rounded-lg"
                            >
                                "Dashboard"
                            </A>
                            <button
                                class="block w-full text-center px-4 py-2 text-sm font-medium text-red-500 border border-red-300 rounded-lg
                                       hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
                                on:click=move |_| {
                                    leptos::task::spawn_local(async move {
                                        crate::ui::auth::logout().await;
                                    });
                                }
                            >
                                "Sign Out"
                            </button>
                        </div>
                    }.into_any()
                }
                _ => {
                    view! {
                        <>
                            <A
                                href="/login"
                                attr:class="block w-full text-center px-4 py-2 text-sm font-medium text-theme-primary border border-theme rounded-lg"
                            >
                                "Sign In"
                            </A>
                            <A
                                href="/register"
                                attr:class="block w-full text-center px-4 py-2 text-sm font-medium text-white bg-accent-primary rounded-lg"
                            >
                                "Sign Up"
                            </A>
                        </>
                    }.into_any()
                }
            }
        }}
    }
}

/// Feature card component
#[component]
fn FeatureCard(
    icon: &'static str,
    title: &'static str,
    description: &'static str,
) -> impl IntoView {
    view! {
        <div class="landing-scroll-animate bg-theme-primary p-6 rounded-xl border border-theme hover:border-accent-primary/50
                    transition-all duration-300 hover:shadow-lg hover:-translate-y-1">
            <div class="w-12 h-12 rounded-lg bg-accent-primary/10 flex items-center justify-center mb-4">
                <FeatureIcon icon=icon />
            </div>
            <h3 class="text-lg font-semibold text-theme-primary mb-2">{title}</h3>
            <p class="text-theme-secondary text-sm leading-relaxed">{description}</p>
        </div>
    }
}

/// SEO Meta tags component using leptos_meta
#[component]
fn SeoMeta() -> impl IntoView {
    view! {
        // Page title
        <Title text="Archischema - Visual Database Schema Designer" />

        // Basic meta tags
        <Meta name="description" content="Design beautiful database schemas visually. Collaborate in real-time with your team. Let AI help you build faster. Free to use, open source." />
        <Meta name="keywords" content="database schema, schema designer, ERD, entity relationship diagram, database design, SQL, visual editor, collaboration, AI assistant" />

        // Open Graph / Facebook
        <Meta property="og:type" content="website" />
        <Meta property="og:url" content="https://archischema.io/" />
        <Meta property="og:title" content="Archischema - Visual Database Schema Designer" />
        <Meta property="og:description" content="Design beautiful database schemas visually. Collaborate in real-time with your team. Let AI help you build faster." />
        <Meta property="og:image" content="https://archischema.io/og-image.png" />

        // Twitter
        <Meta property="twitter:card" content="summary_large_image" />
        <Meta property="twitter:url" content="https://archischema.io/" />
        <Meta property="twitter:title" content="Archischema - Visual Database Schema Designer" />
        <Meta property="twitter:description" content="Design beautiful database schemas visually. Collaborate in real-time with your team. Let AI help you build faster." />
        <Meta property="twitter:image" content="https://archischema.io/og-image.png" />

        // Canonical URL
        <Link rel="canonical" href="https://archischema.io/" />

        // JSON-LD Structured Data (inline script)
        <script type="application/ld+json" inner_html=r#"{"@context":"https://schema.org","@type":"SoftwareApplication","name":"Archischema","applicationCategory":"DeveloperApplication","operatingSystem":"Web","description":"Visual database schema designer with real-time collaboration and AI assistance","url":"https://archischema.io","author":{"@type":"Organization","name":"Archischema"},"offers":{"@type":"Offer","price":"0","priceCurrency":"USD"},"featureList":["Visual drag-and-drop schema editor","Real-time collaboration","AI-powered schema generation","Multiple export formats","Version history"]}"#></script>
    }
}

/// Pricing section component
#[component]
fn PricingSection() -> impl IntoView {
    view! {
        <section id="pricing" class="py-20 px-4 bg-theme-secondary/10">
            <div class="max-w-6xl mx-auto">
                <div class="text-center mb-16 landing-scroll-animate">
                    <h2 class="text-3xl sm:text-4xl font-bold text-theme-primary mb-4">
                        "Simple, Transparent Pricing"
                    </h2>
                    <p class="text-lg text-theme-secondary max-w-2xl mx-auto">
                        "Start for free. Upgrade when you need more power."
                    </p>
                </div>

                <div class="grid md:grid-cols-3 gap-8 max-w-5xl mx-auto">
                    <PricingCard
                        name="Free"
                        price="$0"
                        period="forever"
                        description="Perfect for personal projects and learning"
                        features=vec![
                            ("3 projects", true),
                            ("Basic export (SQL)", true),
                            ("Community support", true),
                            ("AI Assistant (limited)", true),
                            ("Real-time collaboration", false),
                            ("Version history", false),
                            ("Priority support", false),
                        ]
                        cta_text="Get Started"
                        cta_href="/register"
                        highlighted=false
                    />
                    <PricingCard
                        name="Pro"
                        price="$12"
                        period="/month"
                        description="For professionals and small teams"
                        features=vec![
                            ("Unlimited projects", true),
                            ("All export formats", true),
                            ("Email support", true),
                            ("AI Assistant (unlimited)", true),
                            ("Real-time collaboration", true),
                            ("Version history (30 days)", true),
                            ("Priority support", false),
                        ]
                        cta_text="Start Free Trial"
                        cta_href="/register?plan=pro"
                        highlighted=true
                    />
                    <PricingCard
                        name="Team"
                        price="$29"
                        period="/user/month"
                        description="For organizations that need more"
                        features=vec![
                            ("Everything in Pro", true),
                            ("Unlimited team members", true),
                            ("SSO & SAML", true),
                            ("Admin dashboard", true),
                            ("Audit logs", true),
                            ("Version history (unlimited)", true),
                            ("Priority support", true),
                        ]
                        cta_text="Contact Sales"
                        cta_href="/contact"
                        highlighted=false
                    />
                </div>

                <p class="text-center text-theme-tertiary text-sm mt-8 landing-scroll-animate">
                    "All plans include a 14-day free trial. No credit card required."
                </p>
            </div>
        </section>
    }
}

/// Pricing card component
#[component]
fn PricingCard(
    name: &'static str,
    price: &'static str,
    period: &'static str,
    description: &'static str,
    features: Vec<(&'static str, bool)>,
    cta_text: &'static str,
    cta_href: &'static str,
    highlighted: bool,
) -> impl IntoView {
    let card_class = if highlighted {
        "landing-scroll-animate relative bg-theme-primary p-8 rounded-2xl border-2 border-accent-primary shadow-xl scale-105"
    } else {
        "landing-scroll-animate bg-theme-primary p-8 rounded-2xl border border-theme hover:border-theme-secondary transition-colors"
    };

    view! {
        <div class=card_class>
            {if highlighted {
                Some(view! {
                    <div class="absolute -top-4 left-1/2 -translate-x-1/2 px-4 py-1 bg-accent-primary text-white text-sm font-medium rounded-full">
                        "Most Popular"
                    </div>
                })
            } else {
                None
            }}

            <div class="text-center mb-6">
                <h3 class="text-xl font-bold text-theme-primary mb-2">{name}</h3>
                <div class="flex items-baseline justify-center gap-1">
                    <span class="text-4xl font-bold text-theme-primary">{price}</span>
                    <span class="text-theme-secondary">{period}</span>
                </div>
                <p class="text-sm text-theme-secondary mt-2">{description}</p>
            </div>

            <ul class="space-y-3 mb-8">
                {features.into_iter().map(|(feature, included)| {
                    view! {
                        <li class="flex items-center gap-3">
                            {if included {
                                view! {
                                    <Icon name=icons::CHECK class="w-5 h-5 text-green-500 flex-shrink-0" />
                                }.into_any()
                            } else {
                                view! {
                                    <Icon name=icons::X class="w-5 h-5 text-theme-tertiary flex-shrink-0" />
                                }.into_any()
                            }}
                            <span class=if included { "text-theme-primary" } else { "text-theme-tertiary" }>
                                {feature}
                            </span>
                        </li>
                    }
                }).collect_view()}
            </ul>

            <A
                href=cta_href
                attr:class=if highlighted {
                    "block w-full text-center py-3 px-6 bg-accent-primary hover:bg-accent-primary-hover text-white font-semibold rounded-xl transition-colors"
                } else {
                    "block w-full text-center py-3 px-6 border-2 border-theme hover:border-accent-primary text-theme-primary font-semibold rounded-xl transition-colors"
                }
            >
                {cta_text}
            </A>
        </div>
    }
}

/// FAQ section component
#[component]
fn FaqSection() -> impl IntoView {
    view! {
        <section id="faq" class="py-20 px-4">
            <div class="max-w-3xl mx-auto">
                <div class="text-center mb-16 landing-scroll-animate">
                    <h2 class="text-3xl sm:text-4xl font-bold text-theme-primary mb-4">
                        "Frequently Asked Questions"
                    </h2>
                    <p class="text-lg text-theme-secondary">
                        "Got questions? We've got answers."
                    </p>
                </div>

                <div class="space-y-4">
                    <FaqItem
                        question="What is Archischema?"
                        answer="Archischema is a visual database schema designer that lets you create, edit, and export database schemas using an intuitive drag-and-drop interface. It supports real-time collaboration and AI-powered schema generation."
                    />
                    <FaqItem
                        question="Is Archischema free to use?"
                        answer="Yes! Archischema offers a free tier that includes up to 3 projects, basic SQL export, and limited AI assistance. For more features like unlimited projects, real-time collaboration, and full AI access, check out our Pro and Team plans."
                    />
                    <FaqItem
                        question="What databases are supported?"
                        answer="Archischema supports exporting schemas for PostgreSQL, MySQL, SQLite, and Microsoft SQL Server. We're constantly adding support for more databases based on user feedback."
                    />
                    <FaqItem
                        question="How does real-time collaboration work?"
                        answer="With our Pro and Team plans, you can invite team members to your projects. Everyone sees changes in real-time with live cursors, similar to Google Docs. Changes are synced instantly, and you'll never have merge conflicts."
                    />
                    <FaqItem
                        question="Can I self-host Archischema?"
                        answer="Yes! Archischema is open source and can be self-hosted on your own infrastructure. Check out our GitHub repository for installation instructions and Docker images."
                    />
                    <FaqItem
                        question="How does the AI Assistant work?"
                        answer="Our AI Assistant can generate database schemas from natural language descriptions, suggest optimizations, add indexes, and help you modify your schema through conversation. It understands database best practices and can explain its suggestions."
                    />
                    <FaqItem
                        question="Is my data secure?"
                        answer="Absolutely. All data is encrypted in transit and at rest. We don't share your schemas with third parties. For maximum security, you can also self-host Archischema on your own servers."
                    />
                    <FaqItem
                        question="Can I import existing schemas?"
                        answer="Yes! You can import schemas from SQL files, and we're working on direct database connection import. This makes it easy to visualize and modify your existing database structures."
                    />
                </div>
            </div>
        </section>
    }
}

/// FAQ accordion item component
#[component]
fn FaqItem(question: &'static str, answer: &'static str) -> impl IntoView {
    let (is_open, set_is_open) = signal(false);

    view! {
        <div class="landing-scroll-animate border border-theme rounded-xl overflow-hidden">
            <button
                class="w-full px-6 py-4 flex items-center justify-between gap-4 text-left hover:bg-theme-secondary/30 transition-colors"
                on:click=move |_| set_is_open.update(|v| *v = !*v)
                aria-expanded=move || is_open.get()
            >
                <span class="font-semibold text-theme-primary">{question}</span>
                <div
                    class="flex items-center justify-center w-5 h-5 text-theme-tertiary flex-shrink-0 transition-transform duration-300"
                    class=("rotate-180", move || is_open.get())
                >
                    <Icon name=icons::CHEVRON_DOWN class="w-5 h-5" />
                </div>
            </button>
            <div
                class="overflow-hidden transition-all duration-300 max-h-0"
                class:max-h-0=move || !is_open.get()
                class:max-h-96=move || is_open.get()
            >
                <div class="px-6 pb-4 text-theme-secondary leading-relaxed">
                    {answer}
                </div>
            </div>
        </div>
    }
}

/// Feature icon component
#[component]
fn FeatureIcon(icon: &'static str) -> impl IntoView {
    let svg_content = match icon {
        "visual" => view! {
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z" />
        },
        "collab" => view! {
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
        },
        "ai" => view! {
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
        },
        "export" => view! {
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
        },
        "version" => view! {
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
        },
        "secure" => view! {
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
        },
        _ => view! {
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
        },
    };

    view! {
        <svg class="w-6 h-6 text-accent-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor" aria-hidden="true">
            {svg_content}
        </svg>
    }
}

/// AI Interface Mockup with looping animation
#[component]
fn AIInterfaceMockup() -> impl IntoView {
    view! {
        <div class="bg-theme-secondary/50 rounded-2xl border border-theme overflow-hidden shadow-2xl">
            // Header bar
            <div class="flex items-center justify-between px-4 py-3 bg-theme-secondary border-b border-theme">
                <div class="flex items-center gap-3">
                    <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-purple-500 to-pink-500 flex items-center justify-center">
                        <svg class="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" aria-hidden="true">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                  d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
                        </svg>
                    </div>
                    <div>
                        <p class="font-medium text-theme-primary text-sm">"AI Assistant"</p>
                        <p class="text-xs text-theme-tertiary">"Full access - AI can create, modify, and delete schema elements"</p>
                    </div>
                </div>
                <span class="px-3 py-1 text-xs font-medium text-white rounded-md shadow-sm" style="background-color: #2563eb;">"Write"</span>
            </div>

            // Chat area with looping animation
            <div class="p-4 space-y-4 min-h-[350px] max-h-[400px] overflow-y-auto">
                // User message
                <div class="flex justify-end landing-ai-msg-1">
                    <div class="bg-accent-primary text-white px-4 py-2.5 rounded-2xl rounded-br-sm max-w-sm shadow-md">
                        <span class="landing-ai-typing">"Create a schema for a blog with users and posts"</span>
                    </div>
                </div>

                // AI thinking indicator
                <div class="flex justify-start landing-ai-thinking">
                    <div class="flex items-center gap-2 px-4 py-2 rounded-2xl rounded-bl-sm bg-theme-tertiary/20">
                        <Icon name=icons::LOADER class="w-4 h-4 text-accent-primary animate-spin" />
                        <span class="text-sm text-theme-secondary">"Using tools..."</span>
                    </div>
                </div>

                // AI response
                <div class="flex justify-start landing-ai-msg-3">
                    <div class="bg-theme-primary border border-theme rounded-2xl rounded-bl-sm p-4 max-w-md shadow-md">
                        <p class="text-theme-primary mb-3 landing-ai-line-1">
                            "I created two tables: "
                            <code class="px-1.5 py-0.5 bg-accent-primary/20 text-accent-primary rounded text-sm">"users"</code>
                            " and "
                            <code class="px-1.5 py-0.5 bg-accent-primary/20 text-accent-primary rounded text-sm">"posts"</code>
                            "."
                        </p>

                        <div class="space-y-2 text-sm">
                            <p class="text-theme-secondary landing-ai-line-2">"Table "<code class="text-accent-primary">"users"</code>" has:"</p>
                            <ul class="list-disc list-inside text-theme-secondary pl-2 space-y-1">
                                <li class="landing-ai-line-3"><code class="text-accent-primary">"id"</code>" (INT, Primary Key)"</li>
                                <li class="landing-ai-line-3"><code class="text-accent-primary">"username"</code>" (VARCHAR)"</li>
                                <li class="landing-ai-line-3"><code class="text-accent-primary">"email"</code>" (VARCHAR, Unique)"</li>
                            </ul>

                            <p class="text-theme-secondary mt-3 landing-ai-line-4">"Table "<code class="text-accent-primary">"posts"</code>" has:"</p>
                            <ul class="list-disc list-inside text-theme-secondary pl-2 space-y-1">
                                <li class="landing-ai-line-5"><code class="text-accent-primary">"id"</code>" (INT, Primary Key)"</li>
                                <li class="landing-ai-line-5"><code class="text-accent-primary">"user_id"</code>" (INT, FK)"</li>
                                <li class="landing-ai-line-5"><code class="text-accent-primary">"title"</code>" (VARCHAR)"</li>
                            </ul>
                        </div>

                        <div class="mt-3 pt-3 border-t border-theme landing-ai-line-6">
                            <p class="text-sm text-theme-secondary">
                                "Relation: "<code class="text-accent-primary">"posts.user_id"</code>" → "<code class="text-accent-primary">"users.id"</code>
                            </p>
                        </div>
                    </div>
                </div>

                // AI follow-up
                <div class="flex justify-start landing-ai-msg-4">
                    <div class="bg-theme-tertiary/20 text-theme-primary px-4 py-2.5 rounded-2xl rounded-bl-sm max-w-sm">
                        "Would you like me to add timestamps or comments?"
                    </div>
                </div>
            </div>

            // Input area
            <div class="p-4 border-t border-theme bg-theme-secondary/30">
                <div class="flex items-center gap-3">
                    <div class="flex-1 relative">
                        <input
                            type="text"
                            placeholder="Ask about your schema..."
                            class="w-full px-4 py-3 bg-theme-primary border border-theme rounded-xl
                                   text-theme-primary placeholder-theme-tertiary
                                   focus:outline-none focus:ring-2 focus:ring-accent-primary focus:border-transparent"
                            disabled
                            aria-label="AI chat input (demo)"
                        />
                    </div>
                    <button
                        class="p-3 rounded-xl transition-colors shadow-md"
                        style="background-color: #2563eb;"
                        disabled
                        aria-label="Send message (demo)"
                    >
                        <Icon name=icons::SEND class="w-5 h-5 text-white" />
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Animated table component for demonstrations
#[component]
fn AnimatedTable(name: &'static str) -> impl IntoView {
    let columns: Vec<(&str, &str, bool)> = match name {
        "users" => vec![
            ("id", "SERIAL", true),
            ("username", "VARCHAR(50)", false),
            ("email", "VARCHAR(100)", false),
            ("created_at", "TIMESTAMP", false),
        ],
        "users_small" => vec![
            ("id", "INT", true),
            ("username", "VARCHAR", false),
            ("email", "VARCHAR", false),
        ],
        "posts" => vec![
            ("id", "INT", true),
            ("user_id", "INT", false),
            ("title", "VARCHAR", false),
            ("content", "TEXT", false),
        ],
        _ => vec![("id", "SERIAL", true), ("name", "VARCHAR", false)],
    };

    let display_name = if name == "users_small" { "users" } else { name };

    view! {
        <div class="bg-theme-primary rounded-lg border border-theme shadow-xl overflow-hidden min-w-[200px]">
            // Table header with drag handle
            <div class="bg-accent-primary/10 px-4 py-2.5 border-b border-theme flex items-center justify-between gap-2">
                <div class="flex items-center gap-2">
                    <Icon name=icons::TABLE class="w-4 h-4 text-accent-primary" />
                    <span class="font-semibold text-theme-primary">{display_name}</span>
                </div>
                // Drag handle indicator
                <div class="grid grid-cols-3 gap-0.5 opacity-50" aria-hidden="true">
                    <div class="w-1 h-1 rounded-full bg-theme-tertiary"></div>
                    <div class="w-1 h-1 rounded-full bg-theme-tertiary"></div>
                    <div class="w-1 h-1 rounded-full bg-theme-tertiary"></div>
                    <div class="w-1 h-1 rounded-full bg-theme-tertiary"></div>
                    <div class="w-1 h-1 rounded-full bg-theme-tertiary"></div>
                    <div class="w-1 h-1 rounded-full bg-theme-tertiary"></div>
                </div>
            </div>
            // Columns
            <div class="divide-y divide-theme/50">
                {columns.into_iter().map(|(col_name, col_type, is_pk)| {
                    view! {
                        <div class="px-4 py-2 flex items-center gap-2 text-sm hover:bg-theme-secondary/30 transition-colors">
                            {if is_pk {
                                Some(view! {
                                    <Icon name=icons::KEY class="w-3.5 h-3.5 text-yellow-500" />
                                })
                            } else {
                                None
                            }}
                            <span class="text-theme-primary font-medium">{col_name}</span>
                            <span class="text-theme-tertiary text-xs ml-auto">{col_type}</span>
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

/// Animated cursor component
#[component]
fn AnimatedCursor(color: &'static str, label: &'static str) -> impl IntoView {
    view! {
        <div class="relative">
            // Cursor pointer
            <svg
                class="w-5 h-5 drop-shadow-lg"
                viewBox="0 0 24 24"
                fill={color}
                aria-hidden="true"
            >
                <path d="M5.5 3.21V20.8c0 .45.54.67.85.35l4.86-4.86a.5.5 0 0 1 .35-.15h6.87c.48 0 .72-.58.38-.92L6.35 2.85a.5.5 0 0 0-.85.36Z"/>
            </svg>
            // Label
            <div
                class="absolute left-4 top-4 px-2 py-0.5 rounded text-xs font-medium text-white whitespace-nowrap shadow-md"
                style=format!("background-color: {};", color)
            >
                {label}
            </div>
        </div>
    }
}

/// Logo component
#[component]
fn Logo() -> impl IntoView {
    view! {
        <div class="w-10 h-10 bg-gradient-to-br from-accent-primary to-blue-600 rounded-xl
                    flex items-center justify-center shadow-lg">
            <svg class="w-6 h-6 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor" aria-hidden="true">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                      d="M4 7v10c0 2 1 3 3 3h10c2 0 3-1 3-3V7c0-2-1-3-3-3H7C5 4 4 5 4 7z" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                      d="M9 12h6M12 9v6" />
            </svg>
        </div>
    }
}

/// GitHub icon component
#[component]
fn GithubIcon() -> impl IntoView {
    view! {
        <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/>
        </svg>
    }
}

/// Footer component
#[component]
fn Footer() -> impl IntoView {
    view! {
        <footer class="py-12 border-t border-theme bg-theme-primary">
            <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
                <div class="grid grid-cols-1 md:grid-cols-4 gap-8 mb-8">
                    // Brand
                    <div class="md:col-span-2">
                        <div class="flex items-center gap-3 mb-4">
                            <Logo />
                            <span class="text-xl font-bold text-theme-primary">"Archischema"</span>
                        </div>
                        <p class="text-sm text-theme-secondary max-w-md">
                            "Design beautiful database schemas visually. Built with Rust & Leptos for maximum performance."
                        </p>
                    </div>

                    // Product links
                    <div>
                        <h4 class="font-semibold text-theme-primary mb-4">"Product"</h4>
                        <ul class="space-y-2">
                            <li>
                                <A href="/editor/demo" attr:class="text-sm text-theme-secondary hover:text-accent-primary transition-colors">
                                    "Try Demo"
                                </A>
                            </li>
                            <li>
                                <A href="/register" attr:class="text-sm text-theme-secondary hover:text-accent-primary transition-colors">
                                    "Sign Up"
                                </A>
                            </li>
                        </ul>
                    </div>

                    // Resources
                    <div>
                        <h4 class="font-semibold text-theme-primary mb-4">"Resources"</h4>
                        <ul class="space-y-2">
                            <li>
                                <a href="https://github.com/c0st1nus/Archischema" target="_blank" rel="noopener noreferrer"
                                   class="text-sm text-theme-secondary hover:text-accent-primary transition-colors">
                                    "GitHub"
                                </a>
                            </li>
                            <li>
                                <a href="https://github.com/c0st1nus/Archischema/issues" target="_blank" rel="noopener noreferrer"
                                   class="text-sm text-theme-secondary hover:text-accent-primary transition-colors">
                                    "Report Issue"
                                </a>
                            </li>
                        </ul>
                    </div>
                </div>

                // Bottom bar
                <div class="pt-8 border-t border-theme/50 flex flex-col sm:flex-row items-center justify-between gap-4">
                    <span class="text-sm text-theme-tertiary">
                        "© 2025 Archischema. Built with ❤️ using Rust & Leptos."
                    </span>
                    <div class="flex items-center gap-4">
                        <a href="https://github.com/c0st1nus/Archischema" target="_blank" rel="noopener noreferrer"
                           class="text-theme-tertiary hover:text-theme-primary transition-colors"
                           aria-label="GitHub repository">
                            <GithubIcon />
                        </a>
                    </div>
                </div>
            </div>
        </footer>
    }
}

/// CSS styles for landing page animations
#[component]
fn LandingStyles() -> impl IntoView {
    view! {
        <style>
            r#"
            /* Button styles */
            .landing-btn-primary {
                padding: 1rem 2rem;
                font-weight: 600;
                font-size: 1.125rem;
                color: white;
                background-color: #2563eb;
                border-radius: 0.75rem;
                transition: all 0.3s;
                transform: scale(1);
                box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1);
                cursor: pointer;
            }
            .landing-btn-primary:hover {
                transform: scale(1.05);
                background-color: #1d4ed8;
            }

            .landing-btn-secondary {
                padding: 1rem 2rem;
                font-weight: 600;
                font-size: 1.125rem;
                border: 2px solid #9ca3af;
                border-radius: 0.75rem;
                transition: all 0.3s;
                box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
                background-color: #f9fafb;
                color: #374151;
            }
            .dark .landing-btn-secondary {
                background-color: #1f2937;
                border-color: #6b7280;
                color: #e5e7eb;
            }
            .landing-btn-secondary:hover {
                transform: scale(1.05);
                box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.1);
            }

            /* Grid background */
            .landing-grid-bg {
                background-image: radial-gradient(circle, currentColor 1px, transparent 1px);
                background-size: 24px 24px;
            }

            /* Fade in up animation */
            @keyframes landing-fade-in-up {
                from {
                    opacity: 0;
                    transform: translateY(20px);
                }
                to {
                    opacity: 1;
                    transform: translateY(0);
                }
            }

            .landing-fade-in-up {
                animation: landing-fade-in-up 0.6s ease-out forwards;
            }

            .landing-delay-200 {
                animation-delay: 0.2s;
                opacity: 0;
            }

            .landing-delay-400 {
                animation-delay: 0.4s;
                opacity: 0;
            }

            /* Scroll animations */
            .landing-scroll-animate {
                opacity: 0;
                transform: translateY(30px);
                transition: opacity 0.6s ease-out, transform 0.6s ease-out;
            }

            .landing-scroll-animate.visible {
                opacity: 1;
                transform: translateY(0);
            }

            /* Visual Editor - container slide animation */
            @keyframes landing-container-slide {
                0%, 10% {
                    top: 50%;
                    left: calc(50% + 200px);
                    transform: translate(-50%, -50%);
                    opacity: 0;
                }
                20% { opacity: 1; }
                30%, 70% {
                    top: 50%;
                    left: 50%;
                    transform: translate(-50%, -50%);
                    opacity: 1;
                }
                80% { opacity: 1; }
                90%, 100% {
                    top: 50%;
                    left: calc(50% - 200px);
                    transform: translate(-50%, -50%);
                    opacity: 0;
                }
            }

            .landing-visual-editor-container {
                animation: landing-container-slide 8s ease-in-out infinite;
            }

            /* Collaboration animations */
            @keyframes landing-table-left {
                0%, 5% { transform: translateX(-50px) translateY(-50%); opacity: 0; }
                15%, 85% { transform: translateX(0) translateY(-50%); opacity: 1; }
                95%, 100% { transform: translateX(-50px) translateY(-50%); opacity: 0; }
            }

            @keyframes landing-table-right {
                0%, 10% { transform: translateX(50px) translateY(-50%); opacity: 0; }
                20%, 80% { transform: translateX(0) translateY(-50%); opacity: 1; }
                90%, 100% { transform: translateX(50px) translateY(-50%); opacity: 0; }
            }

            @keyframes landing-cursor-fade {
                0%, 15% { opacity: 0; }
                20%, 75% { opacity: 1; }
                85%, 100% { opacity: 0; }
            }

            @keyframes landing-connection-fade {
                0%, 30% { opacity: 0; }
                40%, 60% { opacity: 1; }
                70%, 100% { opacity: 0; }
            }

            .landing-collab-table-left { animation: landing-table-left 10s ease-in-out infinite; }
            .landing-collab-table-right { animation: landing-table-right 10s ease-in-out infinite; }
            .landing-cursor-alice { animation: landing-cursor-fade 10s ease-in-out infinite; }
            .landing-cursor-bob { animation: landing-cursor-fade 10s ease-in-out infinite; animation-delay: 0.5s; }
            .landing-collab-connection { animation: landing-connection-fade 10s ease-in-out infinite; }

            /* AI Demo animations - LOOPED */
            @keyframes landing-ai-message {
                0%, 5% { opacity: 0; transform: translateY(10px); }
                10%, 90% { opacity: 1; transform: translateY(0); }
                95%, 100% { opacity: 0; transform: translateY(10px); }
            }

            @keyframes landing-ai-thinking {
                0%, 15% { opacity: 0; }
                20%, 35% { opacity: 1; }
                40%, 100% { opacity: 0; }
            }

            @keyframes landing-ai-response {
                0%, 35% { opacity: 0; transform: translateY(10px); }
                40%, 90% { opacity: 1; transform: translateY(0); }
                95%, 100% { opacity: 0; transform: translateY(10px); }
            }

            @keyframes landing-ai-typing {
                0%, 5% { width: 0; }
                15%, 90% { width: 100%; }
                95%, 100% { width: 0; }
            }

            @keyframes landing-ai-line {
                0%, 40% { opacity: 0; }
                45%, 90% { opacity: 1; }
                95%, 100% { opacity: 0; }
            }

            @keyframes landing-ai-followup {
                0%, 70% { opacity: 0; transform: translateY(10px); }
                75%, 90% { opacity: 1; transform: translateY(0); }
                95%, 100% { opacity: 0; transform: translateY(10px); }
            }

            .landing-ai-demo.visible .landing-ai-msg-1 {
                animation: landing-ai-message 12s ease-in-out infinite;
            }

            .landing-ai-demo.visible .landing-ai-typing {
                display: inline-block;
                overflow: hidden;
                white-space: nowrap;
                animation: landing-ai-typing 12s steps(45, end) infinite;
            }

            .landing-ai-demo.visible .landing-ai-thinking {
                animation: landing-ai-thinking 12s ease-in-out infinite;
            }

            .landing-ai-demo.visible .landing-ai-msg-3 {
                animation: landing-ai-response 12s ease-in-out infinite;
            }

            .landing-ai-demo.visible .landing-ai-line-1 { animation: landing-ai-line 12s ease-in-out infinite; animation-delay: 0.3s; }
            .landing-ai-demo.visible .landing-ai-line-2 { animation: landing-ai-line 12s ease-in-out infinite; animation-delay: 0.5s; }
            .landing-ai-demo.visible .landing-ai-line-3 { animation: landing-ai-line 12s ease-in-out infinite; animation-delay: 0.7s; }
            .landing-ai-demo.visible .landing-ai-line-4 { animation: landing-ai-line 12s ease-in-out infinite; animation-delay: 0.9s; }
            .landing-ai-demo.visible .landing-ai-line-5 { animation: landing-ai-line 12s ease-in-out infinite; animation-delay: 1.1s; }
            .landing-ai-demo.visible .landing-ai-line-6 { animation: landing-ai-line 12s ease-in-out infinite; animation-delay: 1.3s; }

            .landing-ai-demo.visible .landing-ai-msg-4 {
                animation: landing-ai-followup 12s ease-in-out infinite;
            }
            "#
        </style>
    }
}

/// Script for scroll-triggered animations using IntersectionObserver
#[component]
fn ScrollAnimationScript() -> impl IntoView {
    view! {
        <script>
            r#"
            (function() {
                function initScrollAnimations() {
                    const observer = new IntersectionObserver((entries) => {
                        entries.forEach(entry => {
                            if (entry.isIntersecting) {
                                entry.target.classList.add('visible');
                            }
                        });
                    }, {
                        threshold: 0.1,
                        rootMargin: '0px 0px -50px 0px'
                    });

                    document.querySelectorAll('.landing-scroll-animate').forEach(el => {
                        observer.observe(el);
                    });
                }

                if (document.readyState === 'loading') {
                    document.addEventListener('DOMContentLoaded', initScrollAnimations);
                } else {
                    initScrollAnimations();
                }
            })();
            "#
        </script>
    }
}
