describe("theme smoke", () => {
  it("persists an explicit light selection", () => {
    cy.visit("/#/");
    cy.get('[role="radiogroup"][aria-label="Color theme"]').within(() => {
      cy.get('[role="radio"]').contains("Light").click();
    });
    cy.get("html").should("have.class", "light");
    cy.reload();
    cy.get("html").should("have.class", "light");
  });
});
