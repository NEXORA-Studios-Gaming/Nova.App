import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { RouterProvider } from "react-router";
import { ThemeProvider } from "@/components/nova/theme-provider.tsx";
import { TooltipProvider } from "./components/ui/tooltip.tsx";
import { router } from "./router.tsx";

import "./index.css";

function Providers({ children }: { children: React.ReactNode }) {
    return (
        <ThemeProvider>
            <TooltipProvider>{children}</TooltipProvider>
        </ThemeProvider>
    );
}

createRoot(document.getElementById("root")!).render(
    <StrictMode>
        <Providers>
            <RouterProvider router={router} />
        </Providers>
    </StrictMode>
);
