/**
 * Playwright Visual Validation for Universal LSP Documentation Theme
 *
 * Validates that the slate/navy/pink theme is correctly applied
 * to the generated rustdoc documentation.
 */

const { chromium } = require('playwright');
const path = require('path');

const EXPECTED_COLORS = {
    mainBackground: 'rgb(47, 53, 66)',    // #2f3542
    sidebarBackground: 'rgb(30, 39, 46)', // #1e272e
    linkColor: 'rgb(255, 105, 180)',      // #ff69b4
    headingColor: 'rgb(255, 105, 180)',   // #ff69b4
    codeBackground: 'rgb(30, 39, 46)',    // #1e272e
    textColor: 'rgb(232, 234, 237)',      // #e8eaed
};

async function validateTheme() {
    console.log('🎨 Starting Visual Theme Validation...\n');

    const browser = await chromium.launch({ headless: true });
    const context = await browser.newContext();
    const page = await context.newPage();

    // Load the generated documentation
    const docsPath = path.join(__dirname, '../../target/doc/universal_lsp/index.html');
    const docsUrl = `file://${docsPath}`;

    console.log(`📂 Loading documentation from: ${docsUrl}`);
    await page.goto(docsUrl, { waitUntil: 'networkidle' });

    // Take a screenshot for manual review
    await page.screenshot({ path: 'tests/visual/screenshot_docs_theme.png', fullPage: true });
    console.log('📸 Screenshot saved to: tests/visual/screenshot_docs_theme.png\n');

    const results = {
        passed: [],
        failed: [],
    };

    // Test 1: Main background color
    console.log('🔍 Testing main background color...');
    const mainBgColor = await page.evaluate(() => {
        return getComputedStyle(document.body).backgroundColor;
    });

    if (mainBgColor === EXPECTED_COLORS.mainBackground) {
        console.log(`✅ PASS: Main background is correct (${mainBgColor})`);
        results.passed.push('Main background color');
    } else {
        console.log(`❌ FAIL: Main background is ${mainBgColor}, expected ${EXPECTED_COLORS.mainBackground}`);
        results.failed.push(`Main background (got ${mainBgColor})`);
    }

    // Test 2: Sidebar background color
    console.log('🔍 Testing sidebar background color...');
    const sidebarBgColor = await page.evaluate(() => {
        const sidebar = document.querySelector('.sidebar');
        return sidebar ? getComputedStyle(sidebar).backgroundColor : null;
    });

    if (sidebarBgColor === EXPECTED_COLORS.sidebarBackground) {
        console.log(`✅ PASS: Sidebar background is correct (${sidebarBgColor})`);
        results.passed.push('Sidebar background color');
    } else {
        console.log(`❌ FAIL: Sidebar background is ${sidebarBgColor}, expected ${EXPECTED_COLORS.sidebarBackground}`);
        results.failed.push(`Sidebar background (got ${sidebarBgColor})`);
    }

    // Test 3: Link color
    console.log('🔍 Testing link color...');
    const linkColor = await page.evaluate(() => {
        const link = document.querySelector('a');
        return link ? getComputedStyle(link).color : null;
    });

    if (linkColor === EXPECTED_COLORS.linkColor) {
        console.log(`✅ PASS: Link color is correct (${linkColor})`);
        results.passed.push('Link color');
    } else {
        console.log(`❌ FAIL: Link color is ${linkColor}, expected ${EXPECTED_COLORS.linkColor}`);
        results.failed.push(`Link color (got ${linkColor})`);
    }

    // Test 4: Heading color
    console.log('🔍 Testing heading color...');
    const headingColor = await page.evaluate(() => {
        const heading = document.querySelector('h1');
        return heading ? getComputedStyle(heading).color : null;
    });

    // Note: h1 may have gradient, so we check if it contains pink
    const headingHasPink = headingColor && (
        headingColor.includes('255, 105, 180') ||
        headingColor.includes('255, 20, 147')
    );

    if (headingHasPink) {
        console.log(`✅ PASS: Heading has pink color (${headingColor})`);
        results.passed.push('Heading color');
    } else {
        console.log(`❌ FAIL: Heading color is ${headingColor}, expected pink gradient`);
        results.failed.push(`Heading color (got ${headingColor})`);
    }

    // Test 5: Code block background
    console.log('🔍 Testing code block background...');
    const codeBgColor = await page.evaluate(() => {
        const code = document.querySelector('pre, code');
        return code ? getComputedStyle(code).backgroundColor : null;
    });

    if (codeBgColor === EXPECTED_COLORS.codeBackground) {
        console.log(`✅ PASS: Code background is correct (${codeBgColor})`);
        results.passed.push('Code block background');
    } else {
        console.log(`❌ FAIL: Code background is ${codeBgColor}, expected ${EXPECTED_COLORS.codeBackground}`);
        results.failed.push(`Code background (got ${codeBgColor})`);
    }

    // Test 6: Text color
    console.log('🔍 Testing text color...');
    const textColor = await page.evaluate(() => {
        return getComputedStyle(document.body).color;
    });

    if (textColor === EXPECTED_COLORS.textColor) {
        console.log(`✅ PASS: Text color is correct (${textColor})`);
        results.passed.push('Text color');
    } else {
        console.log(`❌ FAIL: Text color is ${textColor}, expected ${EXPECTED_COLORS.textColor}`);
        results.failed.push(`Text color (got ${textColor})`);
    }

    // Test 7: Font family
    console.log('🔍 Testing custom font...');
    const fontFamily = await page.evaluate(() => {
        return getComputedStyle(document.body).fontFamily;
    });

    const hasCustomFont = fontFamily.includes('Inter') || fontFamily.includes('Fira Sans');

    if (hasCustomFont) {
        console.log(`✅ PASS: Custom font applied (${fontFamily})`);
        results.passed.push('Custom font');
    } else {
        console.log(`❌ FAIL: Font is ${fontFamily}, expected Inter or Fira Sans`);
        results.failed.push(`Font (got ${fontFamily})`);
    }

    // Test 8: Pink scrollbar (check CSS variable)
    console.log('🔍 Testing scrollbar color...');
    const scrollbarColor = await page.evaluate(() => {
        return getComputedStyle(document.documentElement).getPropertyValue('--scrollbar-thumb-background-color').trim();
    });

    if (scrollbarColor === '#ff69b4') {
        console.log(`✅ PASS: Scrollbar color is correct (${scrollbarColor})`);
        results.passed.push('Scrollbar color');
    } else {
        console.log(`❌ FAIL: Scrollbar color is ${scrollbarColor}, expected #ff69b4`);
        results.failed.push(`Scrollbar color (got ${scrollbarColor})`);
    }

    // Summary
    console.log('\n' + '='.repeat(60));
    console.log('📊 VALIDATION SUMMARY');
    console.log('='.repeat(60));
    console.log(`✅ Passed: ${results.passed.length}/${results.passed.length + results.failed.length}`);
    console.log(`❌ Failed: ${results.failed.length}/${results.passed.length + results.failed.length}\n`);

    if (results.passed.length > 0) {
        console.log('✅ Passed tests:');
        results.passed.forEach(test => console.log(`   - ${test}`));
        console.log('');
    }

    if (results.failed.length > 0) {
        console.log('❌ Failed tests:');
        results.failed.forEach(test => console.log(`   - ${test}`));
        console.log('');
    }

    await browser.close();

    // Exit with appropriate code
    process.exit(results.failed.length > 0 ? 1 : 0);
}

// Run the validation
validateTheme().catch(error => {
    console.error('❌ Validation error:', error);
    process.exit(1);
});
