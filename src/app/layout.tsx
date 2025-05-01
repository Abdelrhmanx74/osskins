import { Poppins } from "next/font/google";
import "./globals.css";
import { Toaster } from "sonner";
import { ThemeProvider } from "next-themes";
import { ThemeInitializer } from "@/components/providers/ThemeInitializer";
import { TerminalLogListener } from "@/components/providers/TerminalLogListener";

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
      <body
        className={`${poppins.variable} antialiased flex flex-col h-screen`}
      >
        <ThemeProvider attribute="class" enableSystem>
          <ThemeInitializer />
          {children}
          <Toaster richColors position="bottom-center" />
          <TerminalLogListener />
        </ThemeProvider>
      </body>
    </html>
  );
}
