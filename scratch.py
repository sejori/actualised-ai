# -*- coding: utf-8 -*-
import re

with open("packages/web-ui/src/App.tsx", "r", encoding="utf-8") as f:
    c = f.read()

# Fix the incorrect replacement of </main>;
# It should only be applied to Dashboard. Dashboard ends with </aside>\n    </Show>\n  </main></div>;
# SetupPage ends with </main></div>;
# AuthPage ends with </main></div>;

# Let's revert the </main></div>; and just manually fix Dashboard's end.

c = c.replace('</main></div>;', '</main>;')

# Now apply it ONLY to Dashboard.
# Dashboard's </main> is right before "const SetupPage"
c = c.replace("</main>;\n};\n\nconst SetupPage", "</main></div>;\n};\n\nconst SetupPage")

with open("packages/web-ui/src/App.tsx", "w", encoding="utf-8") as f:
    f.write(c)

