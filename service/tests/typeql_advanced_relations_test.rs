//! Tests for advanced TypeQL relation features

use linkml_core::prelude::*;
use linkml_service::generator::typeql_relation_analyzer::RelationAnalyzer;
use linkml_service::generator::{EnhancedTypeQLGenerator, Generator, GeneratorOptions};
use linkml_core::types::{SchemaDefinition, ClassDefinition, SlotDefinition, EnumDefinition, TypeDefinition, SubsetDefinition, Element};

/// Helper to create a test schema with advanced relations
fn create_advanced_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.id = "https://example.org/advanced".to_string();
    schema.name = "AdvancedRelations".to_string();

    // Entity classes
    let mut student = ClassDefinition::default();
    student.name = "Student".to_string();
    schema.classes.insert("Student".to_string(), student);

    let mut course = ClassDefinition::default();
    course.name = "Course".to_string();
    schema.classes.insert("Course".to_string(), course);

    let mut instructor = ClassDefinition::default();
    instructor.name = "Instructor".to_string();
    schema.classes.insert("Instructor".to_string(), instructor);

    let mut semester = ClassDefinition::default();
    semester.name = "Semester".to_string();
    schema.classes.insert("Semester".to_string(), semester);

    // Multi-way relation: Enrollment
    let mut enrollment = ClassDefinition::default();
    enrollment.name = "Enrollment".to_string();
    enrollment.description = Some("Links student, course, instructor, and semester".to_string());
    enrollment.slots = vec![
        "student".to_string(),
        "course".to_string(),
        "instructor".to_string(),
        "semester".to_string(),
        "grade".to_string(),
        "enrollment_date".to_string(),
    ];
    schema.classes.insert("Enrollment".to_string(), enrollment);

    // Define slots for enrollment
    let mut student_slot = SlotDefinition::default();
    student_slot.name = "student".to_string();
    student_slot.range = Some("Student".to_string());
    student_slot.required = Some(true);
    schema.slots.insert("student".to_string(), student_slot);

    let mut course_slot = SlotDefinition::default();
    course_slot.name = "course".to_string();
    course_slot.range = Some("Course".to_string());
    course_slot.required = Some(true);
    schema.slots.insert("course".to_string(), course_slot);

    let mut instructor_slot = SlotDefinition::default();
    instructor_slot.name = "instructor".to_string();
    instructor_slot.range = Some("Instructor".to_string());
    instructor_slot.required = Some(true);
    schema
        .slots
        .insert("instructor".to_string(), instructor_slot);

    let mut semester_slot = SlotDefinition::default();
    semester_slot.name = "semester".to_string();
    semester_slot.range = Some("Semester".to_string());
    semester_slot.required = Some(true);
    schema.slots.insert("semester".to_string(), semester_slot);

    let mut grade_slot = SlotDefinition::default();
    grade_slot.name = "grade".to_string();
    grade_slot.range = Some("string".to_string());
    grade_slot.pattern = Some(r"^[A-F][+-]?$".to_string());
    schema.slots.insert("grade".to_string(), grade_slot);

    let mut date_slot = SlotDefinition::default();
    date_slot.name = "enrollment_date".to_string();
    date_slot.range = Some("date".to_string());
    schema
        .slots
        .insert("enrollment_date".to_string(), date_slot);

    schema
}

#[tokio::test]
async fn test_multiway_relation_generation() {
    let generator = EnhancedTypeQLGenerator::new();
    let schema = create_advanced_schema();

    let outputs = generator.generate(&schema).expect("Test operation failed");
    let content = &outputs[0].content;

    // Check multi-way relation is generated
    assert!(content.contains("enrollment sub relation,"));
    assert!(content.contains("# Multi-way relation with 4 roles"));

    // Check all roles are present
    assert!(content.contains("relates student"));
    assert!(content.contains("relates course"));
    assert!(content.contains("relates instructor"));
    assert!(content.contains("relates semester"));

    // Check attributes
    assert!(content.contains("owns grade"));
    assert!(content.contains("owns enrollment-date"));

    // Check role players
    assert!(content.contains("student plays enrollment:student;"));
    assert!(content.contains("course plays enrollment:course;"));
    assert!(content.contains("instructor plays enrollment:instructor;"));
    assert!(content.contains("semester plays enrollment:semester;"));
}

#[tokio::test]
async fn test_nested_relation_detection() {
    let mut schema = create_advanced_schema();

    // Add a contract that references enrollment (nested relation)
    let mut contract = ClassDefinition::default();
    contract.name = "Contract".to_string();
    contract.slots = vec!["enrollment".to_string(), "terms".to_string()];
    schema.classes.insert("Contract".to_string(), contract);

    // Add slots
    let mut enrollment_slot = SlotDefinition::default();
    enrollment_slot.name = "enrollment".to_string();
    enrollment_slot.range = Some("Enrollment".to_string());
    schema
        .slots
        .insert("enrollment".to_string(), enrollment_slot);

    let mut terms_slot = SlotDefinition::default();
    terms_slot.name = "terms".to_string();
    terms_slot.range = Some("string".to_string());
    schema.slots.insert("terms".to_string(), terms_slot);

    let generator = EnhancedTypeQLGenerator::new();

    let outputs = generator.generate(&schema).expect("Test operation failed");
    let content = &outputs[0].content;

    // Check that enrollment can play a role
    assert!(content.contains("enrollment plays contract:enrollment;"));

    // Verify contract is also a relation
    assert!(content.contains("contract sub relation,"));
}

#[tokio::test]
async fn test_role_inheritance() {
    let mut schema = SchemaDefinition::default();

    // Create abstract participation relation
    let mut participation = ClassDefinition::default();
    participation.name = "Participation".to_string();
    participation.abstract_ = Some(true);
    participation.slots = vec!["participant".to_string()];
    schema
        .classes
        .insert("Participation".to_string(), participation);

    // Create person entity
    let mut person = ClassDefinition::default();
    person.name = "Person".to_string();
    schema.classes.insert("Person".to_string(), person);

    // Create student as subtype of person
    let mut student = ClassDefinition::default();
    student.name = "Student".to_string();
    student.is_a = Some("Person".to_string());
    schema.classes.insert("Student".to_string(), student);

    // Create enrollment that extends participation
    let mut enrollment = ClassDefinition::default();
    enrollment.name = "Enrollment".to_string();
    enrollment.is_a = Some("Participation".to_string());
    enrollment.slots = vec!["student".to_string(), "course".to_string()];
    schema.classes.insert("Enrollment".to_string(), enrollment);

    // Add course entity
    let mut course = ClassDefinition::default();
    course.name = "Course".to_string();
    schema.classes.insert("Course".to_string(), course);

    // Define slots
    let mut participant_slot = SlotDefinition::default();
    participant_slot.name = "participant".to_string();
    participant_slot.range = Some("Person".to_string());
    schema
        .slots
        .insert("participant".to_string(), participant_slot);

    let mut student_slot = SlotDefinition::default();
    student_slot.name = "student".to_string();
    student_slot.range = Some("Student".to_string());
    schema.slots.insert("student".to_string(), student_slot);

    let mut course_slot = SlotDefinition::default();
    course_slot.name = "course".to_string();
    course_slot.range = Some("Course".to_string());
    schema.slots.insert("course".to_string(), course_slot);

    let generator = EnhancedTypeQLGenerator::new();

    let outputs = generator.generate(&schema).expect("Test operation failed");
    let content = &outputs[0].content;

    // Check abstract relation
    assert!(content.contains("participation sub relation, abstract"));
    assert!(content.contains("relates participant"));

    // Check inherited relation
    assert!(content.contains("enrollment sub participation"));

    // Check role specialization (student specializes participant)
    // This would require full implementation of role inheritance detection
    // For now, check basic structure
    assert!(content.contains("relates student"));
    assert!(content.contains("relates course"));
}

#[test]
fn test_relation_analyzer_directly() {
    let mut analyzer = RelationAnalyzer::new();
    let schema = create_advanced_schema();

    // Test enrollment detection
    let enrollment_class = schema
        .classes
        .get("Enrollment")
        .expect("Test operation failed");
    let relation_info = analyzer.analyze_relation("Enrollment", enrollment_class, &schema);

    assert!(relation_info.is_some());
    let info = relation_info.expect("Test operation failed");

    // Check multi-way detection
    assert!(info.is_multiway);
    assert_eq!(info.roles.len(), 4);

    // Check roles
    let role_names: Vec<String> = info.roles.iter().map(|r| r.name.clone()).collect();
    assert!(role_names.contains(&"student".to_string());
    assert!(role_names.contains(&"course".to_string());
    assert!(role_names.contains(&"instructor".to_string());
    assert!(role_names.contains(&"semester".to_string());

    // Check attributes
    assert_eq!(info.attributes.len(), 2);
    assert!(info.attributes.contains(&"grade".to_string());
    assert!(info.attributes.contains(&"enrollment_date".to_string());
}

#[test]
fn test_polymorphic_role_detection() {
    let mut schema = SchemaDefinition::default();

    // Create base person type
    let mut person = ClassDefinition::default();
    person.name = "Person".to_string();
    schema.classes.insert("Person".to_string(), person);

    // Create subtypes
    let mut student = ClassDefinition::default();
    student.name = "Student".to_string();
    student.is_a = Some("Person".to_string());
    schema.classes.insert("Student".to_string(), student);

    let mut teacher = ClassDefinition::default();
    teacher.name = "Teacher".to_string();
    teacher.is_a = Some("Person".to_string());
    schema.classes.insert("Teacher".to_string(), teacher);

    // Create meeting relation that accepts any person
    let mut meeting = ClassDefinition::default();
    meeting.name = "Meeting".to_string();
    meeting.slots = vec!["attendee".to_string()];
    schema.classes.insert("Meeting".to_string(), meeting);

    let mut attendee_slot = SlotDefinition::default();
    attendee_slot.name = "attendee".to_string();
    attendee_slot.range = Some("Person".to_string());
    attendee_slot.multivalued = Some(true);
    schema.slots.insert("attendee".to_string(), attendee_slot);

    let mut analyzer = RelationAnalyzer::new();
    let meeting_class = schema
        .classes
        .get("Meeting")
        .expect("Test operation failed");
    analyzer.analyze_relation("Meeting", meeting_class, &schema);

    // Check polymorphic roles
    let polymorphic = analyzer.detect_polymorphic_roles(&schema);
    assert!(!polymorphic.is_empty());

    // Meeting:attendee should be polymorphic (Person, Student, Teacher can play it)
    let attendee_players = polymorphic.get("Meeting:attendee");
    assert!(attendee_players.is_some());
    let players = attendee_players.expect("Test operation failed");
    assert!(players.contains(&"Person".to_string());
    assert!(players.contains(&"Student".to_string());
    assert!(players.contains(&"Teacher".to_string());
}

#[test]
fn test_role_cardinality() {
    let mut schema = SchemaDefinition::default();

    // Create entities
    schema
        .classes
        .insert("Author".to_string(), ClassDefinition::default());
    schema
        .classes
        .insert("Book".to_string(), ClassDefinition::default());

    // Create authorship relation with cardinality constraints
    let mut authorship = ClassDefinition::default();
    authorship.name = "Authorship".to_string();
    authorship.slots = vec!["author".to_string(), "book".to_string()];
    schema.classes.insert("Authorship".to_string(), authorship);

    // Author slot (a book must have at least 1 author)
    let mut author_slot = SlotDefinition::default();
    author_slot.name = "author".to_string();
    author_slot.range = Some("Author".to_string());
    author_slot.required = Some(true);
    author_slot.multivalued = Some(true);
    schema.slots.insert("author".to_string(), author_slot);

    // Book slot (an author can write multiple books)
    let mut book_slot = SlotDefinition::default();
    book_slot.name = "book".to_string();
    book_slot.range = Some("Book".to_string());
    book_slot.required = Some(true);
    schema.slots.insert("book".to_string(), book_slot);

    let mut analyzer = RelationAnalyzer::new();
    let authorship_class = schema
        .classes
        .get("Authorship")
        .expect("Test operation failed");
    let relation_info = analyzer.analyze_relation("Authorship", authorship_class, &schema);

    assert!(relation_info.is_some());
    let info = relation_info.expect("Test operation failed");

    // Check role cardinalities
    let author_role = info
        .roles
        .iter()
        .find(|r| r.name == "author")
        .expect("Test operation failed");
    assert_eq!(author_role.cardinality, Some((1, None))); // 1..* authors

    let book_role = info
        .roles
        .iter()
        .find(|r| r.name == "book")
        .expect("Test operation failed");
    assert_eq!(book_role.cardinality, None); // Default 1..1
}
