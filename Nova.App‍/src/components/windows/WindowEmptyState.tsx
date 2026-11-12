import type { ReactNode } from "react";

type WindowEmptyStateProps = {
    icon: ReactNode;
    label: string;
};

export function WindowEmptyState({ icon, label }: WindowEmptyStateProps) {
    return (
        <div className="flex min-h-36 flex-col items-center justify-center gap-3 text-sm text-muted-foreground">
            <span className="flex size-10 items-center justify-center rounded-md bg-muted/70 [&_svg]:size-5">{icon}</span>
            <span>{label}</span>
        </div>
    );
}
