describe("documentation navigation", () => {
  it("loads the Coding-Assistants landing page and opens the docs reader", () => {
    cy.visit("/#/");
    cy.contains("h1", "Coding-Assistants").should("be.visible");
    cy.contains("a", "Read the docs").click();
    cy.location("hash").should("eq", "#/docs");
    cy.contains("h1", "Documentation").should("be.visible");
  });

  it("opens search and navigates to a documentation result", () => {
    cy.visit("/#/");
    cy.get('button[aria-label="Search documentation"]').click();
    cy.get('[role="dialog"][aria-label="Search documentation"]').should("be.visible");
    cy.get('input[placeholder="Search titles, headings, and body"]').type("architecture");
    cy.get('[role="option"] button').first().click();
    cy.location("hash").should("match", /^#\/docs\//);
  });
});
