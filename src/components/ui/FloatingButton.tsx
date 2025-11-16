"use client";

import React, { ReactNode, useEffect } from "react";
import { AnimatePresence, motion } from "framer-motion";

type FloatingButtonProps = {
    className?: string;
    children: ReactNode;
    triggerContent: ReactNode;
};

type FloatingButtonItemProps = {
    children: ReactNode;
};

const list = {
    visible: {
        opacity: 1,
        transition: {
            staggerChildren: 0.03,
            staggerDirection: -1,
        },
    },
    hidden: {
        opacity: 0,
        transition: {
            when: "afterChildren",
            staggerChildren: 0.03,
        },
    },
};

const item = {
    visible: { opacity: 1, y: 0, transition: { duration: 0.12 } },
    hidden: { opacity: 0, y: 6, transition: { duration: 0.1 } },
};

const btn = {
    visible: { rotate: "45deg", transition: { duration: 0.12 } },
    hidden: { rotate: 0, transition: { duration: 0.08 } },
};

function useOnClickOutside<T extends HTMLElement>(ref: React.RefObject<T>, handler: (e: Event) => void) {
    useEffect(() => {
        const listener = (event: Event) => {
            const el = ref?.current;
            if (!el || el.contains(event.target as Node)) return;
            handler(event);
        };
        document.addEventListener("mousedown", listener);
        document.addEventListener("touchstart", listener);
        return () => {
            document.removeEventListener("mousedown", listener);
            document.removeEventListener("touchstart", listener);
        };
    }, [ref, handler]);
}

function FloatingButton({ children, triggerContent, className }: FloatingButtonProps) {
    const ref = React.useRef<HTMLDivElement | null>(null);
    const [isOpen, setIsOpen] = React.useState(false);

    useOnClickOutside(ref as React.RefObject<HTMLDivElement>, () => setIsOpen(false));

    return (
        <div className={`flex flex-col items-center relative ${className ?? ""}`}>
            <AnimatePresence>
                <motion.ul
                    key="list"
                    className="flex flex-col items-center absolute bottom-10 gap-1"
                    initial="hidden"
                    animate={isOpen ? "visible" : "hidden"}
                    variants={list}
                    role="list"
                    style={{ willChange: "transform, opacity" }}
                >
                    {children}
                </motion.ul>
                <motion.div
                    key="button"
                    variants={btn}
                    animate={isOpen ? "visible" : "hidden"}
                    ref={ref}
                    onClick={() => setIsOpen((s) => !s)}
                    className="cursor-pointer"
                    aria-expanded={isOpen}
                    aria-label="Open chroma menu"
                >
                    {triggerContent}
                </motion.div>
            </AnimatePresence>
        </div>
    );
}

function FloatingButtonItem({ children }: FloatingButtonItemProps) {
    return (
        <motion.li variants={item} role="listitem">
            {children}
        </motion.li>
    );
}

export { FloatingButton, FloatingButtonItem };
