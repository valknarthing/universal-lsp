/**
 * Emoji Detection Test for Universal LSP Documentation
 *
 * Tests whether emojis are present in the generated documentation
 * and provides detailed information about where they are found.
 */

const { chromium } = require('playwright');
const path = require('path');

async function detectEmojis() {
    console.log('🔍 EMOJI DETECTION TEST\n');

    const browser = await chromium.launch({ headless: true });
    const context = await browser.newContext();
    const page = await context.newPage();

    // Load the generated documentation
    const docsPath = path.join(__dirname, '../../target/doc/universal_lsp/index.html');
    const docsUrl = `file://${docsPath}`;

    console.log(`📂 Loading: ${docsUrl}\n`);
    await page.goto(docsUrl, { waitUntil: 'networkidle' });

    // Extract all text content from the page
    const pageText = await page.evaluate(() => {
        return document.body.innerText;
    });

    // Regex to match emoji characters (comprehensive emoji ranges)
    const emojiRegex = /[\u{1F300}-\u{1F9FF}\u{2600}-\u{26FF}\u{2700}-\u{27BF}\u{1F000}-\u{1F02F}\u{1F0A0}-\u{1F0FF}\u{1F100}-\u{1F64F}\u{1F680}-\u{1F6FF}\u{1F900}-\u{1F9FF}\u{1FA00}-\u{1FA6F}\u{1FA70}-\u{1FAFF}\u{2B50}\u{2B55}\u{231A}\u{231B}\u{23E9}-\u{23F3}\u{23F8}-\u{23FA}\u{25AA}\u{25AB}\u{25B6}\u{25C0}\u{25FB}-\u{25FE}\u{2934}\u{2935}\u{2B05}-\u{2B07}\u{2B1B}\u{2B1C}\u{3030}\u{303D}\u{3297}\u{3299}]/gu;

    const emojis = pageText.match(emojiRegex) || [];

    console.log('='.repeat(70));
    console.log('EMOJI DETECTION RESULTS');
    console.log('='.repeat(70));

    if (emojis.length === 0) {
        console.log('✅ SUCCESS: No emojis found in documentation!');
        console.log('='.repeat(70));
        await browser.close();
        process.exit(0);
    } else {
        console.log(`❌ FAILURE: Found ${emojis.length} emoji(s) in documentation`);
        console.log(`\nEmojis found: ${[...new Set(emojis)].join(' ')}`);
        console.log('\n' + '-'.repeat(70));
        console.log('DETAILED ANALYSIS');
        console.log('-'.repeat(70));

        // Find where emojis appear in the DOM
        const emojiLocations = await page.evaluate(() => {
            const emojiRegex = /[\u{1F300}-\u{1F9FF}\u{2600}-\u{26FF}\u{2700}-\u{27BF}]/gu;
            const locations = [];

            function checkNode(node, path = '') {
                if (node.nodeType === Node.TEXT_NODE) {
                    const matches = node.textContent.match(emojiRegex);
                    if (matches) {
                        locations.push({
                            path: path || 'body',
                            text: node.textContent.substring(0, 100),
                            emojis: matches,
                            parent: node.parentElement?.tagName,
                            class: node.parentElement?.className
                        });
                    }
                } else if (node.nodeType === Node.ELEMENT_NODE) {
                    const newPath = path + ' > ' + node.tagName.toLowerCase() +
                        (node.className ? '.' + node.className.split(' ')[0] : '');
                    for (let child of node.childNodes) {
                        checkNode(child, newPath);
                    }
                }
            }

            checkNode(document.body);
            return locations;
        });

        console.log(`\nFound emojis in ${emojiLocations.length} location(s):\n`);

        emojiLocations.forEach((loc, idx) => {
            console.log(`${idx + 1}. ${loc.parent}.${loc.class || 'no-class'}`);
            console.log(`   Emojis: ${loc.emojis.join(' ')}`);
            console.log(`   Context: "${loc.text.substring(0, 80)}..."`);
            console.log(`   Path: ${loc.path.substring(0, 100)}`);
            console.log('');
        });

        console.log('-'.repeat(70));
        console.log('RECOMMENDATIONS:');
        console.log('-'.repeat(70));
        console.log('1. Emojis in text content cannot be removed with CSS alone');
        console.log('2. They must be removed from Rust source code doc comments');
        console.log('3. Alternative: Use JavaScript in documentation to filter them');
        console.log('='.repeat(70));

        await browser.close();
        process.exit(1);
    }
}

// Run the detection
detectEmojis().catch(error => {
    console.error('❌ Test error:', error);
    process.exit(1);
});
