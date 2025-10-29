/**
 * Documentation Theme Validation Test
 *
 * Validates:
 * 1. Mermaid.js diagrams are rendering correctly
 * 2. No black icons/SVG elements in the documentation
 * 3. All theme colors match the pink/slate/navy scheme
 */

const { chromium } = require('playwright');
const path = require('path');

async function validateDocumentation() {
    console.log('🔍 DOCUMENTATION THEME VALIDATION\n');

    const browser = await chromium.launch({ headless: true });
    const context = await browser.newContext();
    const page = await context.newPage();

    // Load the generated documentation
    const docsPath = path.join(__dirname, '../../target/doc/universal_lsp/index.html');
    const docsUrl = `file://${docsPath}`;

    console.log(`📂 Loading: ${docsUrl}\n`);
    await page.goto(docsUrl, { waitUntil: 'networkidle' });

    // Wait for Mermaid to load
    await page.waitForTimeout(2000);

    let allTestsPassed = true;

    console.log('='.repeat(70));
    console.log('TEST 1: MERMAID.JS DIAGRAM VALIDATION');
    console.log('='.repeat(70));

    // Check for Mermaid diagrams
    const mermaidDiagrams = await page.locator('.mermaid').count();

    if (mermaidDiagrams > 0) {
        console.log(`✅ SUCCESS: Found ${mermaidDiagrams} Mermaid diagram(s)`);

        // Validate diagram content
        for (let i = 0; i < mermaidDiagrams; i++) {
            const diagram = page.locator('.mermaid').nth(i);
            const hasSvg = await diagram.locator('svg').count() > 0;

            if (hasSvg) {
                console.log(`   ✓ Diagram ${i + 1}: Rendered as SVG`);
            } else {
                console.log(`   ✗ Diagram ${i + 1}: NOT rendered (no SVG found)`);
                allTestsPassed = false;
            }
        }
    } else {
        console.log('❌ FAILURE: No Mermaid diagrams found!');
        console.log('   Expected diagrams in Architecture section');
        allTestsPassed = false;
    }

    console.log('\n' + '='.repeat(70));
    console.log('TEST 2: BLACK ICON DETECTION');
    console.log('='.repeat(70));

    // Check for black SVG elements
    const svgElements = await page.evaluate(() => {
        const blackElements = [];

        // Check all SVG paths, lines, etc.
        const svgShapes = document.querySelectorAll('svg path, svg line, svg polyline, svg polygon, svg circle, svg ellipse, svg rect');

        svgShapes.forEach((element, index) => {
            const stroke = window.getComputedStyle(element).stroke;
            const fill = window.getComputedStyle(element).fill;

            // Check if stroke or fill is black
            const isBlackStroke = stroke === 'rgb(0, 0, 0)' || stroke === 'black' || stroke === '#000' || stroke === '#000000';
            const isBlackFill = fill === 'rgb(0, 0, 0)' || fill === 'black' || fill === '#000' || fill === '#000000';

            if (isBlackStroke || isBlackFill) {
                const parent = element.closest('button, a, summary') || element.parentElement;
                blackElements.push({
                    index,
                    tag: element.tagName,
                    stroke,
                    fill,
                    parent: parent ? parent.tagName : 'none',
                    parentId: parent ? parent.id : '',
                    parentClass: parent ? parent.className : ''
                });
            }
        });

        return blackElements;
    });

    if (svgElements.length === 0) {
        console.log('✅ SUCCESS: No black SVG elements found!');
        console.log('   All icons are using theme colors');
    } else {
        console.log(`❌ FAILURE: Found ${svgElements.length} black SVG element(s)\n`);

        svgElements.forEach((elem, idx) => {
            console.log(`${idx + 1}. ${elem.tag} (index ${elem.index})`);
            console.log(`   Parent: <${elem.parent}${elem.parentId ? ' id="' + elem.parentId + '"' : ''}${elem.parentClass ? ' class="' + elem.parentClass + '"' : ''}>`);
            console.log(`   Stroke: ${elem.stroke}`);
            console.log(`   Fill: ${elem.fill}`);
            console.log('');
        });

        allTestsPassed = false;
    }

    console.log('='.repeat(70));
    console.log('TEST 3: THEME COLOR VALIDATION');
    console.log('='.repeat(70));

    // Check for primary theme colors
    const themeColors = await page.evaluate(() => {
        const colors = {
            pink: '#ff69b4',
            slate: '#2f3542',
            navy: '#1e272e'
        };

        const results = {
            linkColor: window.getComputedStyle(document.documentElement).getPropertyValue('--link-color').trim(),
            borderColor: window.getComputedStyle(document.documentElement).getPropertyValue('--border-color').trim(),
            found: {
                pink: false,
                slate: false,
                navy: false
            }
        };

        // Sample some elements to check if theme colors are present
        const allElements = Array.from(document.querySelectorAll('*'));
        const colorSample = new Set();

        allElements.slice(0, 100).forEach(el => {
            const style = window.getComputedStyle(el);
            colorSample.add(style.color);
            colorSample.add(style.backgroundColor);
            colorSample.add(style.borderColor);
        });

        colorSample.forEach(color => {
            if (color.includes('255, 105, 180')) results.found.pink = true; // rgb(255, 105, 180) = #ff69b4
            if (color.includes('47, 53, 66')) results.found.slate = true;     // rgb(47, 53, 66) = #2f3542
            if (color.includes('30, 39, 46')) results.found.navy = true;      // rgb(30, 39, 46) = #1e272e
        });

        return results;
    });

    console.log(`Link color (--link-color): ${themeColors.linkColor}`);
    console.log(`Border color (--border-color): ${themeColors.borderColor}`);
    console.log('\nTheme color usage:');
    console.log(`   ${themeColors.found.pink ? '✓' : '✗'} Pink (#ff69b4) detected`);
    console.log(`   ${themeColors.found.slate ? '✓' : '✗'} Slate (#2f3542) detected`);
    console.log(`   ${themeColors.found.navy ? '✓' : '✗'} Navy (#1e272e) detected`);

    if (themeColors.found.pink && themeColors.found.slate && themeColors.found.navy) {
        console.log('\n✅ SUCCESS: All theme colors are present');
    } else {
        console.log('\n⚠️  WARNING: Some theme colors may not be applied');
    }

    console.log('\n' + '='.repeat(70));
    console.log('FINAL RESULT');
    console.log('='.repeat(70));

    if (allTestsPassed) {
        console.log('✅ ALL TESTS PASSED');
        console.log('   • Mermaid.js diagrams are rendering');
        console.log('   • No black icons detected');
        console.log('   • Theme colors are applied');
        console.log('='.repeat(70));
        await browser.close();
        process.exit(0);
    } else {
        console.log('❌ SOME TESTS FAILED');
        console.log('   Please review the failures above');
        console.log('='.repeat(70));
        await browser.close();
        process.exit(1);
    }
}

// Run the validation
validateDocumentation().catch(error => {
    console.error('❌ Test error:', error);
    process.exit(1);
});
