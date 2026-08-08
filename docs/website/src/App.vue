<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { marked } from 'marked';
import docsData from './data/docs.json';

interface Doc {
  id: string;
  title: string;
  category: string;
  path: string;
  content: string;
  summary: string;
}

// State
const docs = ref<Doc[]>(docsData as Doc[]);
const searchQuery = ref('');
const currentDocId = ref('welcome');
const sidebarOpen = ref(false);
const theme = ref<'light' | 'dark'>('dark');

// Lifecycle
onMounted(() => {
  // Load saved theme or default to dark
  const savedTheme = localStorage.getItem('theme') as 'light' | 'dark' | null;
  const userPreferredDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
  
  if (savedTheme) {
    theme.value = savedTheme;
  } else if (userPreferredDark) {
    theme.value = 'dark';
  } else {
    theme.value = 'light';
  }
  
  document.documentElement.setAttribute('data-theme', theme.value);

  // Parse URL hash for direct routing if present
  if (window.location.hash) {
    const hashId = window.location.hash.substring(1);
    const matchedDoc = docs.value.find(d => d.id === hashId);
    if (matchedDoc) {
      currentDocId.value = matchedDoc.id;
    }
  }
});

// Watch current doc selection to update hash
const selectDoc = (id: string) => {
  currentDocId.value = id;
  window.location.hash = id;
  sidebarOpen.value = false;
  // Scroll content back to top
  const scroller = document.querySelector('.content-scroller');
  if (scroller) {
    scroller.scrollTop = 0;
  }
};

// Theme management
const toggleTheme = () => {
  theme.value = theme.value === 'light' ? 'dark' : 'light';
  document.documentElement.setAttribute('data-theme', theme.value);
  localStorage.setItem('theme', theme.value);
};

// Search & filter
const filteredDocs = computed(() => {
  if (!searchQuery.value.trim()) {
    return docs.value;
  }
  const query = searchQuery.value.toLowerCase().trim();
  return docs.value.filter(
    doc =>
      doc.title.toLowerCase().includes(query) ||
      doc.category.toLowerCase().includes(query) ||
      doc.content.toLowerCase().includes(query)
  );
});

// Group docs by category
const categorizedDocs = computed(() => {
  const groups: Record<string, Doc[]> = {};
  
  // Initialize standard order
  const categoriesInOrder = ["Welcome", "Getting Started", "Decision Records", "Research", "Reports", "Roadmap & Changelog"];
  categoriesInOrder.forEach(cat => {
    groups[cat] = [];
  });
  
  filteredDocs.value.forEach(doc => {
    if (!groups[doc.category]) {
      groups[doc.category] = [];
    }
    groups[doc.category].push(doc);
  });
  
  // Filter out empty categories
  return Object.keys(groups).reduce<Record<string, Doc[]>>((acc, key) => {
    if (groups[key].length > 0) {
      acc[key] = groups[key];
    }
    return acc;
  }, {});
});

// Active Document
const currentDoc = computed(() => {
  return docs.value.find(doc => doc.id === currentDocId.value) || docs.value[0];
});

// Navigation indices
const currentDocIndex = computed(() => {
  return docs.value.findIndex(doc => doc.id === currentDoc.value.id);
});

const prevDoc = computed(() => {
  if (currentDocIndex.value > 0) {
    return docs.value[currentDocIndex.value - 1];
  }
  return null;
});

const nextDoc = computed(() => {
  if (currentDocIndex.value < docs.value.length - 1) {
    return docs.value[currentDocIndex.value + 1];
  }
  return null;
});

// Headings extraction for active doc (TOC)
const headings = computed(() => {
  if (!currentDoc.value) return [];
  const lines = currentDoc.value.content.split('\n');
  const extracted: { text: string; id: string; level: number }[] = [];
  
  lines.forEach(line => {
    const h2Match = line.match(/^##\s+(.+)$/);
    const h3Match = line.match(/^###\s+(.+)$/);
    
    if (h2Match) {
      const text = h2Match[1].trim();
      const id = text.toLowerCase().replace(/[^\w]+/g, '-').replace(/(^-|-$)/g, '');
      extracted.push({ text, id, level: 2 });
    } else if (h3Match) {
      const text = h3Match[1].trim();
      const id = text.toLowerCase().replace(/[^\w]+/g, '-').replace(/(^-|-$)/g, '');
      extracted.push({ text, id, level: 3 });
    }
  });
  return extracted;
});

// Render markdown with custom alerts preprocessor
const htmlContent = computed(() => {
  if (!currentDoc.value) return '';
  let md = currentDoc.value.content;

  // 1. Heading IDs preprocessor (so they match TOC IDs)
  md = md.replace(/^##\s+(.+)$/gm, (_, text) => {
    const cleanText = text.replace(/\[([^\]]+)\]\([^)]+\)/g, '$1');
    const id = cleanText.toLowerCase().replace(/[^\w]+/g, '-').replace(/(^-|-$)/g, '');
    return `<h2 id="${id}">${text}</h2>`;
  });
  
  md = md.replace(/^###\s+(.+)$/gm, (_, text) => {
    const cleanText = text.replace(/\[([^\]]+)\]\([^)]+\)/g, '$1');
    const id = cleanText.toLowerCase().replace(/[^\w]+/g, '-').replace(/(^-|-$)/g, '');
    return `<h3 id="${id}">${text}</h3>`;
  });

  // 2. Alert Blocks Preprocessor
  // Matches > [!NOTE] followed by blockquote body lines
  const alertRegex = />\s*\[!(NOTE|TIP|WARNING|IMPORTANT|CAUTION)\]\s*\n((?:>\s*.*\n?)*)/gi;
  md = md.replace(alertRegex, (_, type, body) => {
    const cleanBody = body.replace(/^>\s?/gm, '');
    const parsedBody = marked.parse(cleanBody);
    const typeUpper = type.toUpperCase();
    return `<div class="alert-block alert-${typeUpper.toLowerCase()}">
      <div class="alert-header">
        <span class="alert-title">${typeUpper}</span>
      </div>
      <div class="alert-body">${parsedBody}</div>
    </div>`;
  });

  return marked.parse(md);
});

// Smooth scroll to headers
const scrollToHeading = (id: string) => {
  const el = document.getElementById(id);
  if (el) {
    el.scrollIntoView({ behavior: 'smooth' });
  }
};
</script>

<template>
  <div class="portal-layout">
    <!-- Sidebar -->
    <aside class="portal-sidebar" :class="{ open: sidebarOpen }">
      <div class="sidebar-header">
        <div class="logo-glow">P</div>
        <div class="sidebar-title">Polyglot Portal</div>
      </div>
      
      <div class="sidebar-search">
        <div class="search-input-wrapper">
          <svg class="search-icon" xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"></circle><line x1="21" y1="21" x2="16.65" y2="16.65"></line></svg>
          <input 
            v-model="searchQuery"
            type="text" 
            placeholder="Search docs..." 
            class="search-input"
          />
        </div>
      </div>

      <nav class="sidebar-nav">
        <div v-for="(docsInCat, catName) in categorizedDocs" :key="catName" class="nav-category">
          <h4 class="category-title">{{ catName }}</h4>
          <ul class="nav-list">
            <li v-for="doc in docsInCat" :key="doc.id" class="nav-item">
              <a 
                class="nav-link" 
                :class="{ active: currentDocId === doc.id }"
                @click="selectDoc(doc.id)"
              >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path><polyline points="14 2 14 8 20 8"></polyline><line x1="16" y1="13" x2="8" y2="13"></line><line x1="16" y1="17" x2="8" y2="17"></line><polyline points="10 9 9 9 8 9"></polyline></svg>
                <span>{{ doc.title }}</span>
              </a>
            </li>
          </ul>
        </div>
        <div v-if="Object.keys(categorizedDocs).length === 0" style="padding: 16px; text-align: center; color: var(--text-muted);">
          No documents found matching "{{ searchQuery }}"
        </div>
      </nav>

      <div class="sidebar-footer">
        <span class="stats-text">{{ docs.length }} documents portal</span>
      </div>
    </aside>

    <!-- Main Content Area -->
    <main class="portal-main">
      <header class="main-header">
        <div style="display: flex; align-items: center; gap: 12px;">
          <!-- Mobile Sidebar Toggle -->
          <button 
            class="theme-toggle" 
            style="display: inline-flex; @media (min-width: 769px) { display: none; }"
            @click="sidebarOpen = !sidebarOpen"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="3" y1="12" x2="21" y2="12"></line><line x1="3" y1="6" x2="21" y2="6"></line><line x1="3" y1="18" x2="21" y2="18"></line></svg>
          </button>

          <div class="header-breadcrumbs">
            <span class="breadcrumb-cat">{{ currentDoc.category }}</span>
            <span class="breadcrumb-separator">/</span>
            <span class="breadcrumb-page">{{ currentDoc.title }}</span>
          </div>
        </div>

        <div class="header-actions">
          <!-- Theme Toggle -->
          <button class="theme-toggle" @click="toggleTheme" title="Toggle theme">
            <svg v-if="theme === 'light'" xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path></svg>
            <svg v-else xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"></circle><line x1="12" y1="1" x2="12" y2="3"></line><line x1="12" y1="21" x2="12" y2="23"></line><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line><line x1="1" y1="12" x2="3" y2="12"></line><line x1="21" y1="12" x2="23" y2="12"></line><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line></svg>
          </button>
        </div>
      </header>

      <div class="content-grid">
        <div class="content-scroller">
          <article class="content-inner">
            <div class="markdown-body" v-html="htmlContent"></div>
            
            <!-- Previous/Next buttons -->
            <div class="page-nav-buttons">
              <a 
                v-if="prevDoc" 
                class="nav-btn prev hover-lift"
                @click="selectDoc(prevDoc.id)"
              >
                <span class="nav-btn-label">Previous</span>
                <span class="nav-btn-title">← {{ prevDoc.title }}</span>
              </a>
              <div v-else style="flex: 1;"></div>
              
              <a 
                v-if="nextDoc" 
                class="nav-btn next hover-lift"
                @click="selectDoc(nextDoc.id)"
              >
                <span class="nav-btn-label">Next</span>
                <span class="nav-btn-title">{{ nextDoc.title }} →</span>
              </a>
              <div v-else style="flex: 1;"></div>
            </div>
          </article>
        </div>

        <!-- Table of Contents Sidebar -->
        <aside class="toc-sidebar" v-if="headings.length > 0">
          <h4 class="toc-title">On this page</h4>
          <ul class="toc-list">
            <li 
              v-for="heading in headings" 
              :key="heading.id" 
              class="toc-item"
            >
              <a 
                class="toc-link" 
                :class="{ h2: heading.level === 2, h3: heading.level === 3 }"
                @click="scrollToHeading(heading.id)"
              >
                {{ heading.text }}
              </a>
            </li>
          </ul>
        </aside>
      </div>
    </main>
  </div>
</template>
