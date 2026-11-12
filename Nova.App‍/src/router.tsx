import { createBrowserRouter, isRouteErrorResponse, Link, useRouteError } from "react-router";
import { AppBackground } from "@/components/nova/background";
import { AppDock } from "@/components/nova/dock";
import { TopBar } from "@/components/nova/topbar";
import { WindowLayer } from "@/windows";

export function App() {
    return (
        <main className="relative flex h-dvh w-full flex-col overflow-hidden">
            <AppBackground />
            <div className="z-40 shrink-0">
                <TopBar />
            </div>
            <section className="relative min-h-0 flex-1 overflow-hidden">
                <WindowLayer />
                <AppDock />
            </section>
        </main>
    );
}

export function RouteErrorPage() {
    const error = useRouteError();
    const message = isRouteErrorResponse(error)
        ? `${error.status} ${error.statusText}`.trim()
        : "An unexpected error occurred.";

    return (
        <main className="flex min-h-svh items-center justify-center p-6">
            <div className="max-w-md text-center">
                <h1 className="text-xl font-semibold">Unable to open this page</h1>
                <p className="mt-2 text-sm text-muted-foreground">{message}</p>
                <Link className="mt-4 inline-block text-sm text-primary underline-offset-4 hover:underline" to="/">
                    Return home
                </Link>
            </div>
        </main>
    );
}

export const router = createBrowserRouter([
    {
        path: "/",
        Component: App,
        errorElement: <RouteErrorPage />,
    },
]);
