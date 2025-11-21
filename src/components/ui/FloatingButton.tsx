"use client";

import React, { ReactNode, useEffect } from "react";
import { AnimatePresence, motion, MotionProps } from "framer-motion";

type MotionUlProps = React.HTMLAttributes<HTMLUListElement> & MotionProps;
const MotionUl = motion.ul as React.FC<MotionUlProps>;
type MotionDivProps = React.HTMLAttributes<HTMLDivElement> & MotionProps;
const MotionDiv = React.forwardRef<HTMLDivElement, MotionDivProps>((props, ref) => (
    <motion.div ref={ref} {...props} />
));
type MotionLiProps = React.HTMLAttributes<HTMLLIElement> & MotionProps;
const MotionLi = React.forwardRef<HTMLLIElement, MotionLiProps>((props, ref) => (
    <motion.li ref={ref} {...props} />
));

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

    // Ensure the click outside detection covers the *entire* floating button (trigger and items)
    // By attaching the ref to the wrapper container instead of the trigger element, clicks
    // on the children won't be considered "outside" and therefore won't close the menu.
    useOnClickOutside(ref as React.RefObject<HTMLDivElement>, () => setIsOpen(false));

    return (
        <div ref={ref} className={`flex flex-col items-center relative ${className ?? ""}`}>
            <AnimatePresence>
                <MotionUl
                    key="list"
                    initial="hidden"
                    animate={isOpen ? "visible" : "hidden"}
                    variants={list}
                    className="flex flex-col items-center absolute bottom-10 gap-1"
                    role="list"
                    style={{ willChange: "transform, opacity" }}
                >
                    {children}
                </MotionUl>
                <MotionDiv
                    key="button"
                    variants={btn}
                    animate={isOpen ? "visible" : "hidden"}
                    onClick={(e: React.MouseEvent<HTMLDivElement>) => {
                        // Prevent the click bubbling up to parents (e.g., the Card) which may
                        // interpret the click as a select/deselect action.
                        e.stopPropagation();
                        setIsOpen((s) => !s);
                    }}
                    className="cursor-pointer"
                    aria-expanded={isOpen}
                    aria-label="Open chroma menu"
                >
                    {triggerContent}
                </MotionDiv>
            </AnimatePresence>
        </div>
    );
}

function FloatingButtonItem({ children }: FloatingButtonItemProps) {
    return (
        <MotionLi variants={item} role="listitem">
            {children}
        </MotionLi>
    );
}

export { FloatingButton, FloatingButtonItem };
