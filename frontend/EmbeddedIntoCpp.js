const fs = require("fs").promises;
const path = require("path");
const iconv = require("iconv-lite");

const args = process.argv.slice(2);

function getArg(flag, defaultValue) {
    const i = args.indexOf(flag);
    return i !== -1 && args[i + 1] ? args[i + 1] : defaultValue;
}

const MODE = getArg("--mode", "split");
const ROOT = getArg("--root", ".\\public");
const OUT = getArg("--out", ".\\CppOutput\\EmbedFiles");
const MAX_LINE = parseInt(getArg("--maxline", "1023"), 10);
const PATH_PREFIX = getArg("--prefix", "./frontend/public");

if (MODE !== "header" && MODE !== "split") {
    console.error(`[ERROR] Unsupported mode: ${MODE}`);
    process.exit(1);
}

function concatenatPrefix(p) {
    p = p.replace(/\\/g, "/");

    if (PATH_PREFIX) {
        let prefix = PATH_PREFIX.replace(/\\/g, "/");

        if (prefix.endsWith("/")) {
            if (p.startsWith("./")) {
                p = p.slice(2);
            }
        } else {
            if (p.startsWith("./")) {
                p = p.slice(1);
            } else if (!p.startsWith("/")) {
                p = "/" + p;
            }
        }

        return prefix + p;
    }

    if (!p.startsWith("./")) {
        if (p.startsWith("/")) {
            return "." + p;
        }
        return "./" + p;
    }

    return p;
}

function fnv1a(buf) {
    let h = 2166136261;
    for (let i = 0; i < buf.length; i++) {
        h ^= buf[i];
        h = Math.imul(h, 16777619) >>> 0;
    }
    return h >>> 0;
}

function toSignedByte(n) {
    return n > 127 ? n - 256 : n;
}

async function walk(dir, base = "") {
    let res = [];
    const files = await fs.readdir(dir);

    for (const f of files) {
        const full = path.join(dir, f);
        const rel = path.join(base, f).replace(/\\/g, "/");
        const stat = await fs.stat(full);

        if (stat.isDirectory()) {
            res = res.concat(await walk(full, rel));
        } else {
            res.push({ full, rel });
        }
    }

    return res;
}

function makeArray(buffer, varName, isHeader) {
    let lines = [];
    const prefix = `${isHeader ? "static " : ""}const char ${varName}[] = { `;
    const suffix = " };";

    let line = prefix;

    for (let i = 0; i < buffer.length; i++) {
        let token = toSignedByte(buffer[i]).toString() + ",";
        if (line !== prefix) token = " " + token;

        if (line.length + token.length > MAX_LINE) {
            lines.push(line);
            line = token.trimStart();
        } else {
            line += token;
        }
    }

    lines.push(line + suffix);
    return lines.join("\n");
}

async function collect() {
    const files = await walk(ROOT);

    let dataSection = [];
    let buckets = {};

    for (let i = 0; i < files.length; i++) {
        const f = files[i];
        const buffer = await fs.readFile(f.full);
        const name = "F" + (i + 1);

        const pathStr = concatenatPrefix(f.rel);
        const pathGbkStr = iconv.encode(pathStr, "gbk");
        const hash = fnv1a(pathGbkStr);

        console.log(`${pathStr} -> ${hash}`);

        dataSection.push(makeArray(buffer, name, MODE === "header"));

        if (!buckets[hash]) buckets[hash] = [];
        buckets[hash].push({
            path: pathStr,
            name,
            len: buffer.length
        });
    }

    return {
        dataSection: dataSection.join("\n"),
        buckets
    };
}

function generateSwitch(buckets) {
    let out = [];

    out.push("    uint32_t h = fnv1a(path);");
    out.push("");
    out.push("    switch(h)");
    out.push("    {");

    for (const hash in buckets) {
        const list = buckets[hash];

        out.push(`        case ${Number(hash)}:`);

        if (list.length === 1) {
            const e = list[0];
            out.push(`            *outDataPtr=${e.name};`);
            out.push(`            outlen=${e.len};`);
            out.push(`            succ=true;`);
            out.push(`            return;`);
        } else {
            for (let i = 0; i < list.length; i++) {
                const e = list[i];

                if (i === 0)
                    out.push(`            if(strcmp(path,"${e.path}")==0)`);
                else
                    out.push(`            else if(strcmp(path,"${e.path}")==0)`);

                out.push("            {");
                out.push(`                *outDataPtr=${e.name};`);
                out.push(`                outlen=${e.len};`);
                out.push(`                succ=true;`);
                out.push(`                return;`);
                out.push("            }");
            }
        }

        out.push("            break;");
        out.push("");
    }

    out.push("    }");

    return out.join("\n");
}

async function generateCpp() {
    const { dataSection, buckets } = await collect();

    const fnvFunc = `
static uint32_t fnv1a(const char* s)
{
    uint32_t h = 2166136261u;
    while(*s)
    {
        h ^= (unsigned char)(*s++);
        h *= 16777619u;
    }
    return h;
}`;

    const switchCode = generateSwitch(buckets);

    const common = `${dataSection}\n${fnvFunc}\n`;

    if (MODE === "header") {
        const out = `#pragma once
#include <string.h>
#include <stdint.h>
#define USE_EMBEDDED_FILES

${common}
static void GetEmbedFileData(const char* path,int& outlen,const char** outDataPtr,bool& succ)
{
${switchCode}

    *outDataPtr=0;
    outlen=0;
    succ=false;
}
`;


        const outHeaderPath = `${OUT}.h`;
        const outHeaderDir = path.dirname(outHeaderPath);
        await fs.mkdir(outHeaderDir, { recursive: true });

        await fs.writeFile(outHeaderPath, out);
        console.log("Generated:", outHeaderPath);
        return;
    }

    const h = `#pragma once
#include <string.h>
#include <stdint.h>
#define USE_EMBEDDED_FILES

void GetEmbedFileData(const char* path,int& outlen,const char** outDataPtr,bool& succ);
`;

    let outFileName = OUT;
    const lastSlash = Math.max(OUT.lastIndexOf('/'), OUT.lastIndexOf('\\'));
    if (lastSlash !== -1) {
        outFileName = OUT.substring(lastSlash + 1);
    }

    const cpp = `#include "${outFileName}.h"

${common}
void GetEmbedFileData(const char* path,int& outlen,const char** outDataPtr,bool& succ)
{
${switchCode}

    *outDataPtr=0;
    outlen=0;
    succ=false;
}
`;

    const outHeaderDir = path.dirname(OUT);
    await fs.mkdir(outHeaderDir, { recursive: true });
    await fs.writeFile(`${OUT}.h`, h);
    await fs.writeFile(`${OUT}.cpp`, cpp);

    console.log("Generated:", `${OUT}.h + .cpp`);
}

generateCpp().catch(err => {
    console.error(err);
    process.exit(1);
});
