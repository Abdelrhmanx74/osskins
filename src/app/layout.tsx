import { Poppins } from "next/font/google";
import "./globals.css";
import { Toaster } from "sonner";
import { ThemeProvider } from "next-themes";
import { ThemeInitializer } from "@/components/providers/ThemeInitializer";
import { PartyModeProvider } from "@/components/providers/PartyModeProvider";
import { I18nProvider } from "@/lib/i18n";

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
          <I18nProvider>
            <ThemeInitializer />
            <PartyModeProvider>{children}</PartyModeProvider>
            <Toaster richColors position="bottom-center" />
          </I18nProvider>
        </ThemeProvider>
      </body>
    </html>
  );
}
