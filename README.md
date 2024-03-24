# Shelby: Open-Source Management Software for Associations

Managing societies, associations, clubs, and "Vereine" can pose significant challenges, often exacerbated by the lack of suitable software solutions. Many existing options are either confined to local environments, closed-source, or prohibitively expensive for small associations. Shelby aims to bridge this gap by offering a versatile solution that prioritizes affordability, customizability, and ease of deployment.

## Features

- Tailored for Small "Vereine": Shelby is specifically designed to meet the needs of small organizations, ensuring that even modest associations with small budget can benefit from effective management tools.
- High Performance: Leveraging the power of the Rust programming language, Shelby delivers exceptional performance, ensuring a smooth and responsive user experience.
- Effortless Deployment: Shelby simplifies deployment by utilizing containerization technology. With Docker or Podman support, users can easily deploy Shelby on their own servers, eliminating the need for complex setup procedures.
- Efficient File-Based Storage: Enjoy the convenience of centralized document storage coupled with metadata management, all without the overhead of external relational databases. Shelby streamlines data storage, keeping everything organized and accessible.
- Multi-User Support: Facilitate collaborative work with multiple users accessing the system simultaneously. Shelby ensures that your team can collaborate seamlessly without encountering bottlenecks.
- Robust Testing: Built upon a foundation of rigorous testing practices, Shelby is thoroughly vetted to ensure stability, reliability, and security.

## Deployment

### Development Environment
To set up Shelby for development:

1. Clone the repository and open it in Visual Studio Code.
2. Utilize the provided development container for seamless setup.
3. Start the server by running cargo run in the terminal.

### Production Environment
For deploying Shelby in a production environment:

1. Install Docker or any compatible containerization software.
2. Navigate to the root folder of the Shelby repository in your terminal.
3. Build the Docker image using the command: docker build -t shelby:0.1 ..
4. Once the build is complete, launch the container with the command: `docker run -i -p 8080:8080 --mount type=bind,source="FOLDER WHERE database.sqlite WILL BE STORED",target=/data shelby:0.1`
5. Access Shelby through your web browser at http://localhost:8080.

## Contributing
We welcome contributions from the community to enhance Shelby further. Whether you're a developer, designer, or enthusiast, there are various ways to contribute:

- Code Contributions: Help us improve Shelby by fixing bugs, implementing new features, or optimizing existing code. Check out the contribution guidelines for more details.
- Documentation: Improve the project's documentation by clarifying existing content, adding new guides, or translating documentation into different languages.
- Feedback and Suggestions: Share your experiences with Shelby, report bugs, or suggest new features by opening an issue on GitHub.
- Spread the Word: Help us grow the Shelby community by sharing the project with others. Whether through social media, conferences, or local meetups, your support is invaluable.

## License
Shelby is licensed under the GNU Affero General Public License, granting users the freedom to use, and modify the software according to their needs. We believe in fostering an open and collaborative environment where everyone can contribute and benefit from shared knowledge.

## Support
If you encounter any issues while using Shelby or have questions about its functionality, feel free to reach out to us. You can contact the maintainers directly via GitHub issues or join our community forums for assistance. Your feedback helps us improve Shelby and ensures a better experience for all users.