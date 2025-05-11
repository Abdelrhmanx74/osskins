import { Poppins } from "next/font/google";
import "@/app/globals.css";
import { Toaster } from "@/components/ui/sonner";
import { ThemeProvider } from "next-themes";
import { TerminalLogListener } from "@/components/providers/TerminalLogListener";
import { ThemeToneProvider } from "@/components/providers/ThemeToneProvider";

const poppins = Poppins({
  weight: ["400", "500", "600", "700"],
  subsets: ["latin"],
  variable: "--font-poppins",
});

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <title>League Skin Manager</title>
        <meta
          name="description"
          content="A powerful League of Legends skin manager"
        />
      </head>
      <body
        className={`${poppins.variable} antialiased flex flex-col h-screen`}
      >
        <ThemeProvider
          attribute="class"
          defaultTheme="system"
          enableSystem
          disableTransitionOnChange
        >
          <ThemeToneProvider>
            {children}
            <Toaster />
            <TerminalLogListener />
          </ThemeToneProvider>
        </ThemeProvider>
      </body>
    </html>
  );
}
