describe("public-site smoke", () => {
  it("loads the Coding-Assistants landing page with its primary calls to action", () => {
    cy.visit("/#/");
    cy.get("main").should("not.be.empty");
    cy.contains("a", "Read the docs").should("be.visible");
    cy.contains("a", "View GitHub").should("have.attr", "href").and("include", "ACFHarbinger/Coding-Assistants");
  });
});
