export function slugify(value: string): string {
    const lowered = value.toLowerCase();
    let slug = "";
    let prevDash = false;

    for (const ch of lowered) {
        if (/^[a-z0-9]$/i.test(ch)) {
            slug += ch;
            prevDash = false;
        } else {
            if (!prevDash) {
                slug += "-";
                prevDash = true;
            }
        }
    }

    return slug.replace(/^-+|-+$/g, "");
}
