use super::selectors::chain_selector_for_transport;
use super::types::{
    ClickResult, CountResult, ElementResult, FillResult, HighlightResult, Locator, Page,
    PressResult, Result, TextResult, WaitForSelectorResult,
};

impl Locator {
    pub fn page(&self) -> &Page {
        &self.page
    }

    pub fn selector(&self) -> &str {
        &self.selector
    }

    pub fn locator(&self, css_selector: impl Into<String>) -> Locator {
        let child_selector = css_selector.into();
        Locator {
            page: self.page.clone(),
            selector: chain_selector_for_transport(&self.selector, &child_selector),
        }
    }

    pub async fn click(&self) -> Result<ClickResult> {
        self.page.click(self.selector.clone()).await
    }

    pub async fn count(&self) -> Result<CountResult> {
        self.page.count(self.selector.clone()).await
    }

    pub async fn highlight(&self) -> Result<HighlightResult> {
        self.page.highlight(self.selector.clone()).await
    }

    pub async fn focus(&self) -> Result<ElementResult> {
        self.page.focus(self.selector.clone()).await
    }

    pub async fn fill(&self, value: impl Into<String>) -> Result<FillResult> {
        self.page.fill(self.selector.clone(), value.into()).await
    }

    pub async fn hover(&self) -> Result<ElementResult> {
        self.page.hover(self.selector.clone()).await
    }

    pub async fn press(&self, key: impl Into<String>) -> Result<PressResult> {
        self.page.press(self.selector.clone(), key.into()).await
    }

    pub async fn text_content(&self) -> Result<TextResult> {
        self.page.text_content(self.selector.clone()).await
    }

    pub async fn inner_text(&self) -> Result<TextResult> {
        self.page.inner_text(self.selector.clone()).await
    }

    pub async fn wait_for(&self) -> Result<WaitForSelectorResult> {
        self.page.wait_for_selector(self.selector.clone()).await
    }
}
